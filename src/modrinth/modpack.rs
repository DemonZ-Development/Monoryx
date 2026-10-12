use crate::content::{ContentKind, ContentStore, InstalledEntry};
use crate::downloads::{DownloadJob, DownloadManager};
use crate::error::{MonoryxError, Result};
use crate::instance::config::LoaderKind;
use crate::instance::manager::InstanceManager;
use crate::storage::paths::MonoryxPaths;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub type PackProgress = Arc<dyn Fn(String, usize, usize) + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MrpackIndex {
    #[serde(alias = "format_version")]
    pub format_version: u32,
    pub game: String,
    #[serde(default, alias = "version_id")]
    pub version_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    pub dependencies: std::collections::HashMap<String, String>,
    pub files: Vec<MrpackFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MrpackFile {
    pub path: String,
    pub hashes: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub env: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub downloads: Vec<String>,
    #[serde(default, alias = "file_size")]
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct PackInstallReport {
    pub instance_id: String,
    pub files: usize,
    pub skipped: usize,
}

struct PackImportGuard {
    manager: InstanceManager,
    id: String,
    committed: AtomicBool,
}

impl Drop for PackImportGuard {
    fn drop(&mut self) {
        if !self.committed.load(Ordering::Relaxed) {
            if let Err(error) = self.manager.delete(&self.id, true) {
                tracing::error!(
                    "Could not remove incomplete modpack instance {}: {error}",
                    self.id
                );
            }
        }
    }
}

fn loader_from_index(
    deps: &std::collections::HashMap<String, String>,
) -> Result<(LoaderKind, String)> {
    let mut loader = None;
    for (key, kind) in [
        ("fabric-loader", LoaderKind::Fabric),
        ("quilt-loader", LoaderKind::Quilt),
        ("neoforge", LoaderKind::Neoforge),
        ("forge", LoaderKind::Forge),
    ] {
        if let Some(version) = deps.get(key) {
            if loader.is_some() {
                return Err(MonoryxError::Archive(
                    "modpack specifies multiple mod loaders".to_string(),
                ));
            }
            if version.trim().is_empty() {
                return Err(MonoryxError::Archive(format!(
                    "modpack is missing the {key} version"
                )));
            }
            loader = Some((kind, version.clone()));
        }
    }
    Ok(loader.unwrap_or((LoaderKind::Vanilla, String::new())))
}

fn should_install_env(env: &Option<std::collections::HashMap<String, String>>) -> bool {
    let Some(env) = env else {
        return true;
    };
    match env.get("client").map(String::as_str) {
        Some("required") => true,
        Some("unsupported") | Some("optional") => false,
        _ => true,
    }
}

pub async fn install_mrpack(
    dm: &DownloadManager,
    _paths: &MonoryxPaths,
    manager: &InstanceManager,
    mrpack_path: &Path,
    name_override: Option<String>,
    progress: Option<PackProgress>,
) -> Result<PackInstallReport> {
    let report = |msg: String, a: usize, b: usize| {
        if let Some(cb) = &progress {
            cb(msg, a, b);
        }
    };
    report("Reading modpack...".to_string(), 0, 1);
    let pack_path = mrpack_path.to_path_buf();
    let index = tokio::task::spawn_blocking(move || read_index(&pack_path))
        .await
        .map_err(|e| MonoryxError::Archive(e.to_string()))??;
    if index.format_version != 1 {
        return Err(MonoryxError::Archive(format!(
            "unsupported mrpack format version {}",
            index.format_version
        )));
    }
    if index.game != "minecraft" {
        return Err(MonoryxError::Archive(format!(
            "unsupported modpack game {}",
            index.game
        )));
    }
    let mc = index
        .dependencies
        .get("minecraft")
        .filter(|version| !version.trim().is_empty())
        .cloned()
        .ok_or_else(|| {
            MonoryxError::Archive("modpack is missing the minecraft version".to_string())
        })?;
    let (loader, loader_version) = loader_from_index(&index.dependencies)?;
    let name = name_override
        .or(index.name)
        .unwrap_or_else(|| format!("Modpack {mc}"));
    let files: Vec<MrpackFile> = index
        .files
        .into_iter()
        .filter(|f| should_install_env(&f.env))
        .collect();
    for file in &files {
        validate_pack_file(file)?;
    }
    report("Creating instance...".to_string(), 0, 1);
    let cfg = manager.create(name, mc, loader, loader_version)?;
    let guard = Arc::new(PackImportGuard {
        manager: manager.clone(),
        id: cfg.id.clone(),
        committed: AtomicBool::new(false),
    });
    let game = manager.game_dir(&cfg.id);
    let total = files.len();
    let mut skipped = 0usize;
    for (i, file) in files.iter().enumerate() {
        report(
            format!("Downloading {} ({}/{})", file.path, i + 1, total.max(1)),
            i,
            total.max(1),
        );
        let dest = crate::utils::fs::safe_join(&game, &file.path)?;
        if !crate::utils::fs::is_within_root(&game, &dest) {
            return Err(MonoryxError::UnsafePath(file.path.clone()));
        }
        let mut job = DownloadJob::new(&file.path, &file.downloads[0], dest);
        if let Some(sha512) = file.hashes.get("sha512") {
            job = job.with_sha512(sha512.clone());
        }
        if let Some(sha1) = file.hashes.get("sha1") {
            job = job.with_sha1(sha1.clone());
        }
        if let Some(size) = file.file_size {
            job = job.with_size(size);
        }
        if dm.is_valid_existing(&job) {
            skipped += 1;
        }
        dm.download_with_guard(&job, None, guard.clone()).await?;
    }
    report("Extracting overrides...".to_string(), 0, 1);
    let pack_path = mrpack_path.to_path_buf();
    let override_game = game;
    let override_guard = guard.clone();
    tokio::task::spawn_blocking(move || {
        extract_overrides(&pack_path, &override_game)?;
        for file in &files {
            record_pack_file(&override_guard.manager, &override_guard.id, file)?;
        }
        Ok::<(), MonoryxError>(())
    })
    .await
    .map_err(|e| MonoryxError::Archive(e.to_string()))??;
    guard.committed.store(true, Ordering::Relaxed);
    report("Ready".to_string(), 1, 1);
    Ok(PackInstallReport {
        instance_id: cfg.id,
        files: total,
        skipped,
    })
}

fn read_index(mrpack_path: &Path) -> Result<MrpackIndex> {
    let f = std::fs::File::open(mrpack_path)?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| MonoryxError::Archive(e.to_string()))?;
    let mut index_str = String::new();
    {
        let mut entry = zip.by_name("modrinth.index.json").map_err(|_| {
            MonoryxError::Archive("invalid .mrpack: missing modrinth.index.json".to_string())
        })?;
        use std::io::Read as _;
        let mut limited = (&mut entry).take(MAX_INDEX_BYTES + 1);
        limited
            .read_to_string(&mut index_str)
            .map_err(|e| MonoryxError::Archive(e.to_string()))?;
        if index_str.len() as u64 > MAX_INDEX_BYTES {
            return Err(MonoryxError::Archive(
                "modrinth.index.json is too large".to_string(),
            ));
        }
    }
    Ok(serde_json::from_str(&index_str)?)
}

fn validate_pack_file(file: &MrpackFile) -> Result<()> {
    let normalized = file.path.replace('\\', "/");
    crate::utils::fs::safe_join(Path::new("."), &normalized)?;
    for component in normalized.split('/') {
        crate::utils::fs::safe_file_name(component)?;
    }
    if !file.hashes.contains_key("sha512") && !file.hashes.contains_key("sha1") {
        return Err(MonoryxError::Archive(format!(
            "{} has no checksum, so the download cannot be verified",
            file.path
        )));
    }
    for (kind, length) in [("sha512", 128), ("sha1", 40)] {
        if let Some(hash) = file.hashes.get(kind) {
            if hash.len() != length || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(MonoryxError::Archive(format!(
                    "{} has an invalid {kind} checksum",
                    file.path
                )));
            }
        }
    }
    if file.downloads.is_empty() {
        return Err(MonoryxError::Archive(format!(
            "modpack file has no downloads: {}",
            file.path
        )));
    }
    for download in &file.downloads {
        let valid = url::Url::parse(download)
            .is_ok_and(|url| url.scheme() == "https" && url.host_str().is_some());
        if !valid {
            return Err(MonoryxError::Archive(format!(
                "modpack file has an invalid HTTPS download: {download}"
            )));
        }
    }
    Ok(())
}

const MAX_OVERRIDE_FILES: usize = 10_000;
const MAX_OVERRIDE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_INDEX_BYTES: u64 = 8 * 1024 * 1024;

fn extract_overrides(mrpack_path: &Path, game: &Path) -> Result<usize> {
    use std::io::Read as _;
    let f = std::fs::File::open(mrpack_path)?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| MonoryxError::Archive(e.to_string()))?;
    let mut count = 0usize;
    let mut total_bytes = 0u64;
    for prefix in ["overrides/", "client-overrides/"] {
        for i in 0..zip.len() {
            let mut entry = zip
                .by_index(i)
                .map_err(|e| MonoryxError::Archive(e.to_string()))?;
            let name = entry.name().to_string();
            if entry.is_dir() || entry.is_symlink() {
                continue;
            }
            if !name.starts_with(prefix) {
                continue;
            }
            let rel = match name.split_once('/') {
                Some((_, rest)) => rest,
                None => continue,
            };
            if rel.is_empty() {
                continue;
            }
            for component in rel.replace('\\', "/").split('/') {
                crate::utils::fs::safe_file_name(component)?;
            }
            let dest = crate::utils::fs::safe_join(game, rel)?;
            if !crate::utils::fs::is_within_root(game, &dest) {
                return Err(MonoryxError::UnsafePath(name));
            }
            count += 1;
            if count > MAX_OVERRIDE_FILES {
                return Err(MonoryxError::Archive(
                    "Modpack exceeds maximum allowed override file count".to_string(),
                ));
            }
            let uncompressed_size = entry.size();
            total_bytes = total_bytes.checked_add(uncompressed_size).ok_or_else(|| {
                MonoryxError::Archive("Modpack overrides exceed size limit".to_string())
            })?;
            if total_bytes > MAX_OVERRIDE_BYTES {
                return Err(MonoryxError::Archive(
                    "Modpack overrides exceed safe size limit".to_string(),
                ));
            }
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&dest)?;
            let mut limited = (&mut entry).take(uncompressed_size);
            let copied = std::io::copy(&mut limited, &mut out)?;
            if copied != uncompressed_size {
                return Err(MonoryxError::Archive(
                    "Incomplete or corrupt file in modpack overrides".to_string(),
                ));
            }
        }
    }
    Ok(count)
}

fn record_pack_file(manager: &InstanceManager, instance_id: &str, file: &MrpackFile) -> Result<()> {
    let normalized = file.path.replace('\\', "/");
    let file_name = Path::new(&normalized)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file.path.clone());
    let kind = if normalized.starts_with("mods/") {
        ContentKind::Mod
    } else if normalized.starts_with("resourcepacks/") {
        ContentKind::Resourcepack
    } else if normalized.starts_with("shaderpacks/") {
        ContentKind::Shader
    } else {
        return Ok(());
    };
    if normalized.split('/').count() != 2 {
        return Ok(());
    }
    let dest = crate::utils::fs::safe_join(&manager.game_dir(instance_id), &normalized)?;
    let cfg = manager.get(instance_id)?;
    let store = ContentStore::for_instance(&manager.instance_dir(instance_id));
    store.upsert(InstalledEntry {
        file_name,
        kind,
        project_id: None,
        project_slug: None,
        project_title: None,
        version_id: None,
        version_number: None,
        file_hash_sha512: Some(crate::utils::hash::sha512_file(&dest)?),
        file_hash_sha1: Some(crate::utils::hash::sha1_file(&dest)?),
        size: std::fs::metadata(dest)?.len(),
        enabled: true,
        installed_at: chrono::Utc::now().to_rfc3339(),
        loader: cfg.loader.as_str().to_string(),
        game_version: cfg.minecraft_version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    const STANDARD_INDEX: &str = include_str!("fixtures/standard-pack-index.json");

    fn manager() -> (tempfile::TempDir, MonoryxPaths, InstanceManager) {
        let dir = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(dir.path().to_path_buf());
        paths.ensure_all().unwrap();
        let manager = InstanceManager::new(paths.clone());
        (dir, paths, manager)
    }

    fn archive(path: &Path, index: &str, overrides: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("modrinth.index.json", options).unwrap();
        zip.write_all(index.as_bytes()).unwrap();
        for (name, bytes) in overrides {
            zip.start_file(*name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }

    fn empty_index() -> serde_json::Value {
        let mut index: serde_json::Value = serde_json::from_str(STANDARD_INDEX).unwrap();
        index["files"] = serde_json::json!([]);
        index
    }

    #[tokio::test]
    async fn standard_archive_imports_with_client_overrides_and_content_metadata() {
        let (dir, paths, manager) = manager();
        let pack = dir.path().join("standard.mrpack");
        archive(
            &pack,
            STANDARD_INDEX,
            &[
                ("client-overrides/config/options.txt", b"client settings"),
                ("overrides/config/options.txt", b"shared settings"),
                ("client-overrides/mods/fixture.jar", b"client fixture mod"),
                ("server-overrides/config/server.txt", b"server settings"),
            ],
        );
        let cache_manager = manager.clone();
        let progress: PackProgress = Arc::new(move |message, _, _| {
            if message.starts_with("Downloading ") {
                let cfg = cache_manager.list().unwrap().pop().unwrap();
                std::fs::write(
                    cache_manager.mods_dir(&cfg.id).join("fixture.jar"),
                    b"fixture mod",
                )
                .unwrap();
            }
        });
        let dm = DownloadManager::new(reqwest::Client::new(), 1);
        let report = install_mrpack(&dm, &paths, &manager, &pack, None, Some(progress))
            .await
            .unwrap();
        assert_eq!(report.files, 1);
        assert_eq!(report.skipped, 1);
        let cfg = manager.get(&report.instance_id).unwrap();
        assert_eq!(cfg.name, "Standard Format Pack");
        assert_eq!(cfg.minecraft_version, "1.21.1");
        assert_eq!(cfg.loader, LoaderKind::Fabric);
        assert_eq!(cfg.loader_version, "0.16.9");
        let game = manager.game_dir(&cfg.id);
        assert_eq!(
            std::fs::read(game.join("config/options.txt")).unwrap(),
            b"client settings"
        );
        assert!(!game.join("config/server.txt").exists());
        assert!(!game.join("mods/server.jar").exists());
        let store = ContentStore::for_instance(&manager.instance_dir(&cfg.id));
        let content = store.load_result().unwrap();
        assert_eq!(content.entries.len(), 1);
        let entry = &content.entries[0];
        assert_eq!(entry.file_name, "fixture.jar");
        assert_eq!(entry.loader, "fabric");
        assert_eq!(entry.game_version, "1.21.1");
        let final_file = game.join("mods/fixture.jar");
        assert_eq!(entry.size, std::fs::metadata(&final_file).unwrap().len());
        assert_eq!(
            entry.file_hash_sha512,
            Some(crate::utils::hash::sha512_file(&final_file).unwrap())
        );
        assert_eq!(
            entry.file_hash_sha1,
            Some(crate::utils::hash::sha1_file(&final_file).unwrap())
        );
    }

    #[test]
    fn standard_field_names_and_legacy_aliases_preserve_download_sizes() {
        let index: MrpackIndex = serde_json::from_str(STANDARD_INDEX).unwrap();
        assert_eq!(index.format_version, 1);
        assert_eq!(index.version_id.as_deref(), Some("1.0.0"));
        assert_eq!(index.files[0].file_size, Some(11));
        let serialized = serde_json::to_value(&index).unwrap();
        assert_eq!(serialized["formatVersion"], 1);
        assert_eq!(serialized["files"][0]["fileSize"], 11);
        let legacy = STANDARD_INDEX
            .replace("formatVersion", "format_version")
            .replace("versionId", "version_id")
            .replace("fileSize", "file_size");
        let legacy: MrpackIndex = serde_json::from_str(&legacy).unwrap();
        assert_eq!(legacy.files[0].file_size, Some(11));
    }

    #[tokio::test]
    async fn unsupported_games_and_unsafe_download_paths_do_not_create_instances() {
        let (dir, paths, manager) = manager();
        let pack = dir.path().join("invalid.mrpack");
        let dm = DownloadManager::new(reqwest::Client::new(), 1);
        let mut index = empty_index();
        index["game"] = serde_json::json!("other-game");
        archive(&pack, &index.to_string(), &[]);
        assert!(install_mrpack(&dm, &paths, &manager, &pack, None, None)
            .await
            .is_err());
        assert!(manager.list().unwrap().is_empty());
        let mut index: serde_json::Value = serde_json::from_str(STANDARD_INDEX).unwrap();
        index["files"][0]["path"] = serde_json::json!("../escaped.jar");
        archive(&pack, &index.to_string(), &[]);
        assert!(install_mrpack(&dm, &paths, &manager, &pack, None, None)
            .await
            .is_err());
        assert_eq!(std::fs::read_dir(paths.instances_dir()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn failed_override_extraction_removes_the_incomplete_instance() {
        let (dir, paths, manager) = manager();
        let pack = dir.path().join("unsafe-overrides.mrpack");
        archive(
            &pack,
            &empty_index().to_string(),
            &[
                ("overrides/options.txt", b"partial"),
                ("client-overrides/../../escape.txt", b"unsafe"),
            ],
        );
        let dm = DownloadManager::new(reqwest::Client::new(), 1);
        assert!(install_mrpack(&dm, &paths, &manager, &pack, None, None)
            .await
            .is_err());
        assert_eq!(std::fs::read_dir(paths.instances_dir()).unwrap().count(), 0);
        assert!(!paths.root().join("escape.txt").exists());
    }

    #[test]
    fn pending_extraction_retains_the_instance_until_the_last_import_guard_drops() {
        let (_dir, paths, manager) = manager();
        let cfg = manager
            .create(
                "Incomplete".into(),
                "1.21.1".into(),
                LoaderKind::Vanilla,
                String::new(),
            )
            .unwrap();
        let guard = Arc::new(PackImportGuard {
            manager,
            id: cfg.id.clone(),
            committed: AtomicBool::new(false),
        });
        let extraction = guard.clone();
        drop(guard);
        assert!(paths.instance_dir(&cfg.id).exists());
        drop(extraction);
        assert!(!paths.instance_dir(&cfg.id).exists());
    }
}
