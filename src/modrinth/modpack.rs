use crate::content::{ContentKind, ContentStore, InstalledEntry};
use crate::downloads::{DownloadJob, DownloadManager};
use crate::error::{MonoryxError, Result};
use crate::instance::config::LoaderKind;
use crate::instance::manager::InstanceManager;
use crate::storage::paths::MonoryxPaths;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub type PackProgress = Arc<dyn Fn(String, usize, usize) + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpackIndex {
    pub format_version: u32,
    pub game: String,
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    pub dependencies: std::collections::HashMap<String, String>,
    pub files: Vec<MrpackFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpackFile {
    pub path: String,
    pub hashes: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub env: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub downloads: Vec<String>,
    #[serde(default)]
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct PackInstallReport {
    pub instance_id: String,
    pub files: usize,
    pub skipped: usize,
}

fn loader_from_index(deps: &std::collections::HashMap<String, String>) -> (LoaderKind, String) {
    for (k, v) in deps {
        match k.as_str() {
            "fabric-loader" => return (LoaderKind::Fabric, v.clone()),
            "quilt-loader" => return (LoaderKind::Quilt, v.clone()),
            "neoforge" => return (LoaderKind::Neoforge, v.clone()),
            "forge" => return (LoaderKind::Forge, v.clone()),
            _ => {}
        }
    }
    (LoaderKind::Vanilla, String::new())
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
    paths: &MonoryxPaths,
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
    let f = std::fs::File::open(mrpack_path)?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| MonoryxError::Archive(e.to_string()))?;
    let mut index_str = String::new();
    {
        let mut entry = zip.by_name("modrinth.index.json").map_err(|_| {
            MonoryxError::Archive("invalid .mrpack: missing modrinth.index.json".to_string())
        })?;
        use std::io::Read as _;
        entry
            .read_to_string(&mut index_str)
            .map_err(|e| MonoryxError::Archive(e.to_string()))?;
    }
    let index: MrpackIndex = serde_json::from_str(&index_str)?;
    if index.format_version != 1 {
        return Err(MonoryxError::Archive(format!(
            "unsupported mrpack format version {}",
            index.format_version
        )));
    }
    let mc = index
        .dependencies
        .get("minecraft")
        .cloned()
        .ok_or_else(|| {
            MonoryxError::Archive("modpack is missing the minecraft version".to_string())
        })?;
    let (loader, loader_version) = loader_from_index(&index.dependencies);
    let name = name_override
        .or(index.name.clone())
        .unwrap_or_else(|| format!("Modpack {}", mc));
    report("Creating instance...".to_string(), 0, 1);
    let cfg = manager.create(name, mc.clone(), loader, loader_version.clone())?;
    manager.ensure_game_dirs(&cfg.id)?;
    let game = manager.game_dir(&cfg.id);
    report("Extracting overrides...".to_string(), 0, 1);
    extract_overrides(mrpack_path, &game)?;
    let files: Vec<&MrpackFile> = index
        .files
        .iter()
        .filter(|f| should_install_env(&f.env))
        .collect();
    let total = files.len();
    let mut done = 0usize;
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
        if dest.exists() {
            if let Some(sha512) = file.hashes.get("sha512") {
                if crate::utils::hash::sha512_file(&dest)
                    .unwrap_or_default()
                    .to_lowercase()
                    == sha512.to_lowercase()
                {
                    skipped += 1;
                    done += 1;
                    continue;
                }
            } else if let Some(sha1) = file.hashes.get("sha1") {
                if crate::utils::hash::sha1_file(&dest)
                    .unwrap_or_default()
                    .to_lowercase()
                    == sha1.to_lowercase()
                {
                    skipped += 1;
                    done += 1;
                    continue;
                }
            }
        }
        let url = file.downloads.first().ok_or_else(|| {
            MonoryxError::Archive(format!("modpack file has no downloads: {}", file.path))
        })?;
        if !url.starts_with("https://") {
            return Err(MonoryxError::UnsafePath(format!(
                "blocked non-https URL: {url}"
            )));
        }
        let mut job = DownloadJob::new(&file.path, url, dest.clone());
        if let Some(sha512) = file.hashes.get("sha512") {
            job = job.with_sha512(sha512.clone());
        } else if let Some(sha1) = file.hashes.get("sha1") {
            job = job.with_sha1(sha1.clone());
        }
        if let Some(size) = file.file_size {
            job = job.with_size(size);
        }
        dm.download(&job, None).await?;
        record_pack_file(manager, &cfg.id, file)?;
        done += 1;
    }
    let instance_id = cfg.id.clone();
    let mut cfg = manager.get(&instance_id)?;
    cfg.loader_version = loader_version;
    manager.save(&cfg)?;
    let _ = paths;
    report("Ready".to_string(), 1, 1);
    Ok(PackInstallReport {
        instance_id,
        files: done,
        skipped,
    })
}

fn extract_overrides(mrpack_path: &Path, game: &Path) -> Result<usize> {
    let f = std::fs::File::open(mrpack_path)?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| MonoryxError::Archive(e.to_string()))?;
    let mut count = 0usize;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| MonoryxError::Archive(e.to_string()))?;
        let name = entry.name().to_string();
        if entry.is_dir() || entry.is_symlink() {
            continue;
        }
        if !(name.starts_with("overrides/") || name.starts_with("client-overrides/")) {
            continue;
        }
        let rel = match name.split_once('/') {
            Some((_, rest)) => rest,
            None => continue,
        };
        if rel.is_empty() {
            continue;
        }
        let dest = crate::utils::fs::safe_join(game, rel)?;
        if !crate::utils::fs::is_within_root(game, &dest) {
            return Err(MonoryxError::UnsafePath(name));
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&dest)?;
        std::io::copy(&mut entry, &mut out)?;
        count += 1;
    }
    Ok(count)
}

fn record_pack_file(manager: &InstanceManager, instance_id: &str, file: &MrpackFile) -> Result<()> {
    let file_name = PathBuf::from(&file.path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file.path.clone());
    let kind = if file.path.starts_with("mods/") {
        ContentKind::Mod
    } else if file.path.starts_with("resourcepacks/") {
        ContentKind::Resourcepack
    } else if file.path.starts_with("shaderpacks/") {
        ContentKind::Shader
    } else {
        return Ok(());
    };
    let store = ContentStore::for_instance(&manager.instance_dir(instance_id));
    store.upsert(InstalledEntry {
        file_name,
        kind,
        project_id: None,
        project_slug: None,
        project_title: None,
        version_id: None,
        version_number: None,
        file_hash_sha512: file.hashes.get("sha512").cloned(),
        file_hash_sha1: file.hashes.get("sha1").cloned(),
        size: file.file_size.unwrap_or(0),
        enabled: true,
        installed_at: chrono::Utc::now().to_rfc3339(),
        loader: String::new(),
        game_version: String::new(),
    })
}
