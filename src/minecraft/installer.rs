use crate::downloads::{DownloadJob, DownloadManager};
use crate::error::{MonoryxError, Result};
use crate::minecraft::libraries::{artifact_location, current_natives_key, library_applies};
use crate::minecraft::version::VersionJson;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallPhase {
    Metadata,
    ClientJar,
    Libraries,
    Natives,
    Assets,
    Logging,
    Verifying,
    Ready,
}

impl InstallPhase {
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Metadata => "Resolving metadata...",
            Self::ClientJar => "Downloading client...",
            Self::Libraries => "Downloading libraries...",
            Self::Natives => "Extracting natives...",
            Self::Assets => "Downloading assets...",
            Self::Logging => "Installing logging config...",
            Self::Verifying => "Verifying...",
            Self::Ready => "Ready",
        }
    }
}

pub type PhaseCallback = Arc<dyn Fn(InstallPhase, usize, usize) + Send + Sync>;

const LIBRARIES_BASE: &str = "https://libraries.minecraft.net";

pub async fn install_version(
    dm: &DownloadManager,
    paths: &crate::storage::paths::MonoryxPaths,
    version: &VersionJson,
    on_phase: Option<PhaseCallback>,
) -> Result<InstallReport> {
    crate::utils::fs::safe_file_name(&version.id)?;
    if let Some(jar) = &version.jar {
        crate::utils::fs::safe_file_name(jar)?;
    }
    let report = |p: InstallPhase, a: usize, b: usize| {
        if let Some(cb) = &on_phase {
            cb(p, a, b);
        }
    };
    report(InstallPhase::Metadata, 0, 1);

    let versions_dir = paths.versions_dir();
    let version_dir = versions_dir.join(&version.id);
    std::fs::create_dir_all(&version_dir)?;

    let version_json_path = version_dir.join(format!("{}.json", version.id));
    let pretty = serde_json::to_string_pretty(version)?;
    crate::utils::fs::atomic_write(&version_json_path, pretty.as_bytes())?;

    report(InstallPhase::ClientJar, 0, 1);
    let client_jar = version_dir.join(format!(
        "{}.jar",
        version.jar.as_deref().unwrap_or(&version.id)
    ));

    if let Some(dls) = &version.downloads {
        if let Some(client) = dls.get("client") {
            let job = DownloadJob::new("minecraft client", &client.url, client_jar.clone())
                .with_sha1(client.sha1.clone())
                .with_size(client.size);
            dm.download(&job, None).await?;
        } else {
            return Err(MonoryxError::VersionNotFound(
                "version JSON is missing the client download entry".to_string(),
            ));
        }
    } else {
        return Err(MonoryxError::VersionNotFound(
            "version JSON is missing downloads".to_string(),
        ));
    }

    report(InstallPhase::Libraries, 0, version.libraries.len().max(1));
    let libraries_dir = paths.libraries_dir();
    let counter = Arc::new(AtomicUsize::new(0));
    let total_libs = version
        .libraries
        .iter()
        .filter(|l| library_applies(l))
        .count();
    let mut lib_jobs: Vec<DownloadJob> = Vec::new();
    let mut native_jars: Vec<(PathBuf, Vec<String>)> = Vec::new();

    for lib in &version.libraries {
        if !library_applies(lib) {
            continue;
        }

        if let Some(d) = &lib.downloads {
            if let Some(a) = &d.artifact {
                let dest = crate::utils::fs::safe_join(&libraries_dir, &a.path)?;
                lib_jobs.push(
                    DownloadJob::new(&lib.name, &a.url, dest)
                        .with_sha1(a.sha1.clone())
                        .with_size(a.size),
                );
            }

            if let Some(natives) = &lib.natives {
                if let Some(classifier) = current_natives_key(natives) {
                    if let Some(map) = &d.classifiers {
                        if let Some(art) = map.get(&classifier) {
                            let dest = crate::utils::fs::safe_join(&libraries_dir, &art.path)?;
                            lib_jobs.push(
                                DownloadJob::new(
                                    format!("{} (natives)", lib.name),
                                    &art.url,
                                    dest.clone(),
                                )
                                .with_sha1(art.sha1.clone())
                                .with_size(art.size),
                            );
                            let excludes = lib
                                .extract
                                .as_ref()
                                .map(|e| e.exclude.clone())
                                .unwrap_or_default();
                            native_jars.push((dest, excludes));
                            continue;
                        }
                    }
                }
            }
        } else {
            if lib.natives.is_some() {
                if let Some(natives) = &lib.natives {
                    if let Some(classifier_suffix) = current_natives_key(natives) {
                        let coord = format!("{}:{classifier_suffix}", lib.name);
                        if let Some(path) = crate::minecraft::manifest::maven_coord_to_path(&coord)
                        {
                            let base = lib.url.as_deref().unwrap_or(LIBRARIES_BASE);
                            let url = format!("{}/{path}", base.trim_end_matches('/'));
                            let dest = crate::utils::fs::safe_join(&libraries_dir, &path)?;
                            lib_jobs.push(DownloadJob::new(&lib.name, url, dest.clone()));
                            let excludes = lib
                                .extract
                                .as_ref()
                                .map(|e| e.exclude.clone())
                                .unwrap_or_default();
                            native_jars.push((dest, excludes));
                        }
                        continue;
                    }
                }
            }
            if let Some((path, url)) = artifact_location(lib, LIBRARIES_BASE) {
                let dest = crate::utils::fs::safe_join(&libraries_dir, &path)?;
                lib_jobs.push(DownloadJob::new(&lib.name, url, dest));
            }
        }
    }

    {
        let mut handles = Vec::new();
        for job in lib_jobs {
            let dm = dm.clone();
            let counter = counter.clone();
            let on_phase = on_phase.clone();
            handles.push(tokio::spawn(async move {
                let r = dm.download(&job, None).await;
                let n = counter.fetch_add(1, Ordering::SeqCst) + 1;
                if let Some(cb) = &on_phase {
                    cb(InstallPhase::Libraries, n, total_libs.max(1));
                }
                r
            }));
        }
        for h in handles {
            h.await
                .map_err(|e| MonoryxError::Download(e.to_string()))??;
        }
    }

    report(InstallPhase::Natives, 0, 1);
    let natives_dir = version_dir.join("natives");

    let existing: Vec<(PathBuf, Vec<String>)> = native_jars
        .into_iter()
        .filter(|(p, _)| p.exists())
        .collect();
    if !existing.is_empty() {
        let out_dir = natives_dir.clone();
        tokio::task::spawn_blocking(move || {
            crate::minecraft::natives::extract_natives(&existing, &out_dir)
        })
        .await
        .map_err(|e| MonoryxError::Archive(e.to_string()))??;
    } else {
        std::fs::create_dir_all(&natives_dir)?;
    }

    report(InstallPhase::Logging, 0, 1);
    if let Some(logging) = &version.logging {
        if let Some(client) = logging.get("client").and_then(|c| c.get("file")) {
            if let (Some(id), Some(sha1), Some(size), Some(url)) = (
                client.get("id").and_then(|v| v.as_str()),
                client.get("sha1").and_then(|v| v.as_str()),
                client.get("size").and_then(|v| v.as_u64()),
                client.get("url").and_then(|v| v.as_str()),
            ) {
                crate::utils::fs::safe_file_name(id)?;
                if !url.starts_with("https://") {
                    return Err(MonoryxError::VersionNotFound(
                        "version JSON requested an insecure logging config URL".to_string(),
                    ));
                }
                let dest = version_dir.join(id);
                let job = DownloadJob::new("logging config", url, dest)
                    .with_sha1(sha1.to_string())
                    .with_size(size);
                dm.download(&job, None).await?;
            }
        }
    }

    if let Some(idx) = &version.asset_index {
        report(InstallPhase::Assets, 0, 1);
        let cb: Arc<dyn Fn(usize, usize) + Send + Sync> = {
            let on_phase = on_phase.clone();
            Arc::new(move |a, b| {
                if let Some(cb) = &on_phase {
                    cb(InstallPhase::Assets, a, b.max(1));
                }
            })
        };
        crate::minecraft::assets::install_assets(
            dm,
            &paths.assets_dir(),
            &idx.id,
            &idx.url,
            &idx.sha1,
            idx.size,
            Some(cb),
        )
        .await?;
    }

    report(InstallPhase::Verifying, 1, 1);
    report(InstallPhase::Ready, 1, 1);

    Ok(InstallReport {
        version_id: version.id.clone(),
        version_dir,
        client_jar,
        natives_dir,
    })
}

#[derive(Debug, Clone)]
pub struct InstallReport {
    pub version_id: String,
    pub version_dir: PathBuf,
    pub client_jar: PathBuf,
    pub natives_dir: PathBuf,
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verify {
    Quick,
    Full,
}

pub fn validate_install(
    paths: &crate::storage::paths::MonoryxPaths,
    version: &VersionJson,
    verify: Verify,
) -> Vec<String> {
    let mut problems = Vec::new();
    if crate::utils::fs::safe_file_name(&version.id).is_err()
        || version
            .jar
            .as_deref()
            .is_some_and(|jar| crate::utils::fs::safe_file_name(jar).is_err())
    {
        problems.push("version metadata contains an unsafe file name".to_string());
        return problems;
    }
    let version_dir = paths.versions_dir().join(&version.id);
    let jar_name = version.jar.as_deref().unwrap_or(&version.id);
    let jar = version_dir.join(format!("{jar_name}.jar"));
    match std::fs::metadata(&jar) {
        Ok(m) => {
            if let Some(dls) = &version.downloads {
                if let Some(c) = dls.get("client") {
                    if m.len() != c.size {
                        problems.push(format!("client jar size mismatch ({} bytes)", m.len()));
                    } else if verify == Verify::Full {
                        let actual =
                            crate::utils::hash::sha1_file(&jar).unwrap_or_else(|_| "?".into());
                        if actual.to_lowercase() != c.sha1.to_lowercase() {
                            problems.push("client jar hash mismatch".to_string());
                        }
                    }
                }
            }
        }
        Err(_) => problems.push("client jar missing".to_string()),
    }
    for lib in &version.libraries {
        if !library_applies(lib) {
            continue;
        }
        if let Some(d) = &lib.downloads {
            if let Some(a) = &d.artifact {
                if let Ok(path) = crate::utils::fs::safe_join(&paths.libraries_dir(), &a.path) {
                    check_lib_file(&path, a.size, &a.sha1, &mut problems, &lib.name, verify);
                } else {
                    problems.push(format!("library {} has an unsafe path", lib.name));
                }
            }
            if let Some(natives) = &lib.natives {
                if let Some(classifier) = current_natives_key(natives) {
                    if let Some(artifact) = d
                        .classifiers
                        .as_ref()
                        .and_then(|items| items.get(&classifier))
                    {
                        if let Ok(path) =
                            crate::utils::fs::safe_join(&paths.libraries_dir(), &artifact.path)
                        {
                            check_lib_file(
                                &path,
                                artifact.size,
                                &artifact.sha1,
                                &mut problems,
                                &lib.name,
                                verify,
                            );
                        } else {
                            problems
                                .push(format!("library {} has an unsafe native path", lib.name));
                        }
                    }
                }
            }
        } else {
            let native_path =
                lib.natives
                    .as_ref()
                    .and_then(current_natives_key)
                    .and_then(|classifier| {
                        crate::minecraft::manifest::maven_coord_to_path(&format!(
                            "{}:{classifier}",
                            lib.name
                        ))
                    });
            let path = native_path
                .or_else(|| artifact_location(lib, LIBRARIES_BASE).map(|(path, _)| path));
            if let Some(path) = path {
                match crate::utils::fs::safe_join(&paths.libraries_dir(), &path) {
                    Ok(location) if location.is_file() => {}
                    Ok(_) => problems.push(format!("library {} missing", lib.name)),
                    Err(_) => problems.push(format!("library {} has an unsafe path", lib.name)),
                }
            }
        }
    }

    if let Some(idx) = &version.asset_index {
        if crate::utils::fs::safe_file_name(&idx.id).is_err() {
            problems.push("asset index has an unsafe name".to_string());
            return problems;
        }
        let p = paths
            .assets_dir()
            .join("indexes")
            .join(format!("{}.json", idx.id));
        if verify == Verify::Quick {
            if !p.is_file() {
                problems.push(format!("asset index {} missing", idx.id));
            }
            return problems;
        }
        match std::fs::read(&p) {
            Ok(bytes) => {
                if let Ok(index) =
                    serde_json::from_slice::<crate::minecraft::assets::AssetIndex>(&bytes)
                {
                    for object in index.objects.values() {
                        if object.hash.len() != 40
                            || !object.hash.bytes().all(|b| b.is_ascii_hexdigit())
                        {
                            problems
                                .push("asset index contains an invalid object hash".to_string());
                            break;
                        }
                        let Some(prefix) = object.hash.get(..2) else {
                            problems
                                .push("asset index contains an invalid object hash".to_string());
                            break;
                        };
                        let path = paths
                            .assets_dir()
                            .join("objects")
                            .join(prefix)
                            .join(&object.hash);
                        if std::fs::metadata(path).map_or(true, |meta| meta.len() != object.size) {
                            problems.push(format!(
                                "asset object {} missing or incomplete",
                                object.hash
                            ));
                            if problems.len() >= 10 {
                                break;
                            }
                        }
                    }
                } else {
                    problems.push(format!("asset index {} is corrupt", idx.id));
                }
            }
            Err(_) => problems.push(format!("asset index {} missing", idx.id)),
        }
    }
    problems
}

fn check_lib_file(
    path: &Path,
    size: u64,
    sha1: &str,
    problems: &mut Vec<String>,
    name: &str,
    verify: Verify,
) {
    match std::fs::metadata(path) {
        Ok(m) if m.len() == size => {
            if verify == Verify::Full {
                let actual = crate::utils::hash::sha1_file(path).unwrap_or_default();
                if actual.to_lowercase() != sha1.to_lowercase() {
                    problems.push(format!("library {name} hash mismatch"));
                }
            }
        }
        Ok(m) => problems.push(format!(
            "library {name} size mismatch ({} != {size})",
            m.len()
        )),
        Err(_) => problems.push(format!("library {name} missing")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minecraft::version::AssetIndexRef;
    use crate::storage::paths::MonoryxPaths;

    const JAR: &[u8] = b"pretend this is a version jar";
    const LIB: &[u8] = b"pretend this is a library jar";

    struct Fixture {
        _temp: tempfile::TempDir,
        paths: MonoryxPaths,
        version: VersionJson,
    }

    fn version_from(
        client_sha1: &str,
        client_size: u64,
        lib_sha1: &str,
        lib_size: u64,
    ) -> VersionJson {
        serde_json::from_value(serde_json::json!({
            "id": "1.21.1",
            "downloads": {
                "client": { "sha1": client_sha1, "size": client_size, "url": "c" }
            },
            "libraries": [{
                "name": "com.example:lib:1.0",
                "downloads": {
                    "artifact": { "path": "com/example/lib/1.0/lib-1.0.jar", "sha1": lib_sha1, "size": lib_size, "url": "l" }
                }
            }]
        }))
        .expect("fixture version json")
    }

    fn fixture() -> Fixture {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().into());

        let version_dir = paths.versions_dir().join("1.21.1");
        std::fs::create_dir_all(&version_dir).unwrap();
        let jar = version_dir.join("1.21.1.jar");
        std::fs::write(&jar, JAR).unwrap();

        let lib_rel = "com/example/lib/1.0/lib-1.0.jar";
        let lib = paths.libraries_dir().join(lib_rel);
        std::fs::create_dir_all(lib.parent().unwrap()).unwrap();
        std::fs::write(&lib, LIB).unwrap();

        let version = version_from(
            &crate::utils::hash::sha1_file(&jar).unwrap(),
            JAR.len() as u64,
            &crate::utils::hash::sha1_file(&lib).unwrap(),
            LIB.len() as u64,
        );
        Fixture {
            _temp: temp,
            paths,
            version,
        }
    }

    fn corrupt_of_same_len(original: &[u8]) -> Vec<u8> {
        original
            .iter()
            .map(|b| if *b == b'a' { b'b' } else { b'a' })
            .collect()
    }

    #[test]
    fn a_healthy_install_passes_both_tiers() {
        let f = fixture();
        assert!(validate_install(&f.paths, &f.version, Verify::Quick).is_empty());
        assert!(validate_install(&f.paths, &f.version, Verify::Full).is_empty());
    }

    #[test]
    fn quick_tier_checks_size_but_not_content() {
        let f = fixture();
        let jar = f.paths.versions_dir().join("1.21.1").join("1.21.1.jar");
        let lib = f
            .paths
            .libraries_dir()
            .join("com/example/lib/1.0/lib-1.0.jar");

        std::fs::write(&jar, corrupt_of_same_len(JAR)).unwrap();
        std::fs::write(&lib, corrupt_of_same_len(LIB)).unwrap();
        assert_eq!(
            std::fs::metadata(&jar).unwrap().len(),
            JAR.len() as u64,
            "the corruption must preserve size, or the test proves nothing"
        );

        assert!(
            validate_install(&f.paths, &f.version, Verify::Quick).is_empty(),
            "size-correct files must pass the quick tier"
        );

        let full = validate_install(&f.paths, &f.version, Verify::Full);
        assert!(
            full.iter().any(|p| p.contains("client jar hash mismatch")),
            "full tier must catch the wrong client jar: {full:?}"
        );
        assert!(
            full.iter()
                .any(|p| p == "library com.example:lib:1.0 hash mismatch"),
            "full tier must catch the wrong library: {full:?}"
        );
    }

    #[test]
    fn both_tiers_catch_a_truncated_download() {
        let f = fixture();
        let jar = f.paths.versions_dir().join("1.21.1").join("1.21.1.jar");
        std::fs::write(&jar, b"short").unwrap();

        for verify in [Verify::Quick, Verify::Full] {
            let problems = validate_install(&f.paths, &f.version, verify);
            assert!(
                problems
                    .iter()
                    .any(|p| p.contains("client jar size mismatch")),
                "{verify:?} must catch a truncated client jar: {problems:?}"
            );
        }
    }

    #[test]
    fn both_tiers_catch_a_missing_library() {
        let f = fixture();
        std::fs::remove_file(
            f.paths
                .libraries_dir()
                .join("com/example/lib/1.0/lib-1.0.jar"),
        )
        .unwrap();

        for verify in [Verify::Quick, Verify::Full] {
            let problems = validate_install(&f.paths, &f.version, verify);
            assert!(
                problems.iter().any(|p| p.contains("missing")),
                "{verify:?} must catch a missing library: {problems:?}"
            );
        }
    }

    #[test]
    fn quick_tier_only_checks_that_the_asset_index_exists() {
        let f = fixture();
        let mut v = f.version.clone();
        v.asset_index = Some(AssetIndexRef {
            id: "5".into(),
            sha1: crate::utils::hash::sha1_bytes(b"anything"),
            size: 1,
            total_size: None,
            url: String::new(),
        });

        let problems = validate_install(&f.paths, &v, Verify::Quick);
        assert!(
            problems.iter().any(|p| p.contains("asset index 5 missing")),
            "{problems:?}"
        );
    }

    #[test]
    fn quick_tier_never_parses_the_asset_index() {
        let f = fixture();
        let mut v = f.version.clone();
        v.asset_index = Some(AssetIndexRef {
            id: "5".into(),
            sha1: "0".repeat(40),
            size: 1,
            total_size: None,
            url: String::new(),
        });
        let index = f.paths.assets_dir().join("indexes").join("5.json");
        std::fs::create_dir_all(index.parent().unwrap()).unwrap();
        std::fs::write(&index, b"{ this is not json").unwrap();

        assert!(
            validate_install(&f.paths, &v, Verify::Quick).is_empty(),
            "quick must not read the index, so corruption there is invisible to it"
        );
        let full = validate_install(&f.paths, &v, Verify::Full);
        assert!(
            full.iter().any(|p| p.contains("corrupt")),
            "full must parse the index and report it: {full:?}"
        );
    }

    #[test]
    fn verify_level_maps_onto_the_installer_tier() {
        use crate::config::VerifyLevel;
        assert_eq!(VerifyLevel::Quick.as_verify(), Verify::Quick);
        assert_eq!(VerifyLevel::Full.as_verify(), Verify::Full);
    }
}
