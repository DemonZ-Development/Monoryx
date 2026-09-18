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
                let dest = libraries_dir.join(&a.path);
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
                            let dest = libraries_dir.join(&art.path);
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
                            let dest = libraries_dir.join(&path);
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
                let dest = libraries_dir.join(&path);
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
pub fn validate_install(
    paths: &crate::storage::paths::MonoryxPaths,
    version: &VersionJson,
) -> Vec<String> {
    let mut problems = Vec::new();
    let version_dir = paths.versions_dir().join(&version.id);
    let jar_name = version.jar.as_deref().unwrap_or(&version.id);
    let jar = version_dir.join(format!("{jar_name}.jar"));
    match std::fs::metadata(&jar) {
        Ok(m) => {
            if let Some(dls) = &version.downloads {
                if let Some(c) = dls.get("client") {
                    if m.len() != c.size {
                        problems.push(format!("client jar size mismatch ({} bytes)", m.len()));
                    } else {
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
                check_lib_file(
                    &paths.libraries_dir().join(&a.path),
                    a.size,
                    &a.sha1,
                    &mut problems,
                    &lib.name,
                );
            }
        }
    }

    if let Some(idx) = &version.asset_index {
        let p = paths
            .assets_dir()
            .join("indexes")
            .join(format!("{}.json", idx.id));
        if !p.exists() {
            problems.push(format!("asset index {} missing", idx.id));
        }
    }
    problems
}

fn check_lib_file(path: &Path, size: u64, sha1: &str, problems: &mut Vec<String>, name: &str) {
    match std::fs::metadata(path) {
        Ok(m) if m.len() == size => {
            let actual = crate::utils::hash::sha1_file(path).unwrap_or_default();
            if actual.to_lowercase() != sha1.to_lowercase() {
                problems.push(format!("library {name} hash mismatch"));
            }
        }
        Ok(m) => problems.push(format!(
            "library {name} size mismatch ({} != {size})",
            m.len()
        )),
        Err(_) => problems.push(format!("library {name} missing")),
    }
}
