use crate::error::{MonoryxError, Result};
use crate::instance::config::InstanceConfig;
use crate::instance::manager::InstanceManager;
use serde::{Deserialize, Serialize};
use std::io::{Read as _, Write as _};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_FILES: usize = 100_000;
const MAX_BYTES: u64 = 20 * 1024 * 1024 * 1024;
const PREVIEW_LIMIT: u64 = 512 * 1024;

#[derive(Debug, Clone)]
pub struct WorldInfo {
    pub name: String,
    pub display_name: String,
    pub version: Option<String>,
    pub mode: Option<String>,
    pub last_played: Option<SystemTime>,
    pub bytes: Option<u64>,
    pub icon: Option<crate::app::events::DecodedImage>,
    pub modified: SystemTime,
}

#[derive(Debug, Clone)]
pub struct WorldBackup {
    pub file_name: String,
    pub world_name: String,
    pub modified: SystemTime,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub relative: PathBuf,
    pub is_dir: bool,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub struct WorldSnapshot {
    pub worlds: Vec<WorldInfo>,
    pub backups: Vec<WorldBackup>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupManifest {
    format_version: u32,
    world_name: String,
}

fn instance_game(manager: &InstanceManager, id: &str) -> Result<PathBuf> {
    uuid::Uuid::parse_str(id).map_err(|_| MonoryxError::UnsafePath(id.to_string()))?;
    manager.get(id)?;
    Ok(manager.game_dir(id))
}

fn direct_world(manager: &InstanceManager, id: &str, name: &str) -> Result<PathBuf> {
    crate::utils::fs::safe_file_name(name)?;
    let saves = instance_game(manager, id)?.join("saves");
    let path = saves.join(name);
    let canonical_saves = saves.canonicalize()?;
    let canonical_world = path.canonicalize()?;
    if canonical_world.parent() != Some(canonical_saves.as_path())
        || !canonical_world.join("level.dat").is_file()
    {
        return Err(MonoryxError::UnsafePath(name.to_string()));
    }
    Ok(canonical_world)
}

fn backup_dir(manager: &InstanceManager, id: &str) -> Result<PathBuf> {
    let game = instance_game(manager, id)?;
    Ok(game.parent().unwrap_or(&game).join("world-backups"))
}

pub fn scan(manager: &InstanceManager, id: &str) -> Result<WorldSnapshot> {
    let size_deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let saves = instance_game(manager, id)?.join("saves");
    let mut worlds = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&saves) {
        for entry in entries.flatten().take(2000) {
            if !entry.file_type().is_ok_and(|ty| ty.is_dir()) {
                continue;
            }
            let path = entry.path();
            if !path.join("level.dat").is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let modified = std::fs::metadata(path.join("level.dat"))
                .and_then(|meta| meta.modified())
                .unwrap_or(UNIX_EPOCH);
            let metadata = crate::minecraft::nbt::read_file(&path.join("level.dat"), true);
            let data = metadata.as_ref().and_then(|root| root.get("Data"));
            let text = |key| {
                data.and_then(|data| data.get(key))
                    .and_then(crate::minecraft::nbt::Value::text)
            };
            let number = |key| {
                data.and_then(|data| data.get(key))
                    .and_then(crate::minecraft::nbt::Value::number)
            };
            let display_name = text("LevelName")
                .filter(|name| !name.trim().is_empty())
                .unwrap_or(&name)
                .to_string();
            let version = data
                .and_then(|data| data.get("Version"))
                .and_then(|version| version.get("Name"))
                .and_then(crate::minecraft::nbt::Value::text)
                .map(str::to_string);
            let mode = if number("hardcore") == Some(1) {
                Some("Hardcore")
            } else {
                match number("GameType") {
                    Some(0) => Some("Survival"),
                    Some(1) => Some("Creative"),
                    Some(2) => Some("Adventure"),
                    Some(3) => Some("Spectator"),
                    _ => None,
                }
            }
            .map(str::to_string);
            let last_played = number("LastPlayed")
                .filter(|time| *time > 0)
                .and_then(|time| {
                    UNIX_EPOCH.checked_add(std::time::Duration::from_millis(time as u64))
                });
            let bytes = folder_bytes(&path, size_deadline);
            let icon = read_world_icon(&path.join("icon.png"));
            worlds.push(WorldInfo {
                name,
                display_name,
                version,
                mode,
                last_played,
                bytes,
                icon,
                modified,
            });
        }
    }
    worlds.sort_by(|a, b| {
        b.last_played
            .unwrap_or(b.modified)
            .cmp(&a.last_played.unwrap_or(a.modified))
            .then_with(|| a.name.cmp(&b.name))
    });
    let mut backups = Vec::new();
    if let Ok(entries) = std::fs::read_dir(backup_dir(manager, id)?) {
        for entry in entries.flatten().take(2000) {
            if !entry.file_type().is_ok_and(|ty| ty.is_file())
                || entry.path().extension().and_then(|ext| ext.to_str()) != Some("zip")
            {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy().to_string();
            let Ok(file) = std::fs::File::open(entry.path()) else {
                continue;
            };
            let Ok(mut archive) = zip::ZipArchive::new(file) else {
                continue;
            };
            let Ok(member) = archive.by_name("monoryx-world.json") else {
                continue;
            };
            let mut json = String::new();
            if member.take(4096).read_to_string(&mut json).is_err() {
                continue;
            }
            let Ok(manifest) = serde_json::from_str::<BackupManifest>(&json) else {
                continue;
            };
            if manifest.format_version != 1 {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            backups.push(WorldBackup {
                file_name,
                world_name: manifest.world_name,
                modified: meta.modified().unwrap_or(UNIX_EPOCH),
                bytes: meta.len(),
            });
        }
    }
    backups.sort_by_key(|entry| std::cmp::Reverse(entry.modified));
    Ok(WorldSnapshot { worlds, backups })
}

fn folder_bytes(root: &Path, deadline: std::time::Instant) -> Option<u64> {
    let mut stack = vec![root.to_path_buf()];
    let mut count = 0;
    let mut bytes = 0_u64;
    while let Some(dir) = stack.pop() {
        if std::time::Instant::now() >= deadline {
            return None;
        }
        for entry in std::fs::read_dir(dir).ok()? {
            let entry = entry.ok()?;
            count += 1;
            if count % 32 == 0 && std::time::Instant::now() >= deadline {
                return None;
            }
            if count > MAX_FILES {
                return None;
            }
            let ty = entry.file_type().ok()?;
            if ty.is_symlink() {
                continue;
            }
            if ty.is_dir() {
                stack.push(entry.path());
            } else if ty.is_file() {
                bytes = bytes.checked_add(entry.metadata().ok()?.len())?;
            }
        }
    }
    Some(bytes)
}

fn read_world_icon(path: &Path) -> Option<crate::app::events::DecodedImage> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.len() > 2 * 1024 * 1024 {
        return None;
    }
    let mut reader = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(1024);
    limits.max_image_height = Some(1024);
    limits.max_alloc = Some(8 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().ok()?.thumbnail(64, 64).to_rgba8();
    Some(crate::app::events::DecodedImage {
        width: image.width() as usize,
        height: image.height() as usize,
        pixels: image.into_raw(),
    })
}

fn collect_files(root: &Path) -> Result<Vec<(PathBuf, PathBuf, u64)>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut total = 0u64;
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_symlink() {
                return Err(MonoryxError::Archive(
                    "Linked files are not supported in world backups or imports".to_string(),
                ));
            }
            if ty.is_dir() {
                stack.push(entry.path());
            } else if ty.is_file() {
                let len = entry.metadata()?.len();
                total = total
                    .checked_add(len)
                    .ok_or_else(|| MonoryxError::Archive("Folder is too large".to_string()))?;
                if total > MAX_BYTES || files.len() >= MAX_FILES {
                    return Err(MonoryxError::Archive(
                        "Folder exceeds the backup or import limit".to_string(),
                    ));
                }
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|error| MonoryxError::Archive(error.to_string()))?
                    .to_path_buf();
                files.push((entry.path(), relative, len));
            }
        }
    }
    files.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(files)
}

fn copy_limited(src: &Path, dst: &Path, budget: &mut (usize, u64)) -> Result<()> {
    let files = collect_files(src)?;
    let bytes = files
        .iter()
        .try_fold(0_u64, |total, (_, _, size)| total.checked_add(*size))
        .ok_or_else(|| MonoryxError::Archive("Import is too large".to_string()))?;
    if budget.0.saturating_add(files.len()) > MAX_FILES
        || budget.1.saturating_add(bytes) > MAX_BYTES
    {
        return Err(MonoryxError::Archive(
            "Import exceeds the file or size limit".to_string(),
        ));
    }
    budget.0 += files.len();
    budget.1 += bytes;
    std::fs::create_dir_all(dst)?;
    for (file, relative, _) in files {
        let target = dst.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(file, target)?;
    }
    Ok(())
}

fn unique_destination(parent: &Path, stem: &str) -> PathBuf {
    let mut candidate = parent.join(stem);
    let mut suffix = 2;
    while candidate.exists() {
        candidate = parent.join(format!("{stem} {suffix}"));
        suffix += 1;
    }
    candidate
}

pub fn back_up(manager: &InstanceManager, id: &str, name: &str) -> Result<String> {
    let world = direct_world(manager, id, name)?;
    let files = collect_files(&world)?;
    let dir = backup_dir(manager, id)?;
    std::fs::create_dir_all(&dir)?;
    let file_name = format!(
        "{}-{}-{}.zip",
        name,
        chrono::Local::now().format("%Y%m%d-%H%M%S"),
        &uuid::Uuid::new_v4().to_string()[..8]
    );
    let dest = dir.join(&file_name);
    let part = dest.with_extension("zip.part");
    let result = (|| -> Result<()> {
        let file = std::fs::File::create(&part)?;
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("monoryx-world.json", opts)
            .map_err(|error| MonoryxError::Archive(error.to_string()))?;
        let manifest = BackupManifest {
            format_version: 1,
            world_name: name.to_string(),
        };
        zip.write_all(serde_json::to_string(&manifest)?.as_bytes())?;
        for (path, relative, _) in files {
            let archive_path = format!("world/{}", relative.to_string_lossy().replace('\\', "/"));
            zip.start_file(archive_path, opts)
                .map_err(|error| MonoryxError::Archive(error.to_string()))?;
            std::io::copy(&mut std::fs::File::open(path)?, &mut zip)?;
        }
        zip.finish()
            .map_err(|error| MonoryxError::Archive(error.to_string()))?;
        std::fs::rename(&part, &dest)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result.map(|()| file_name)
}

pub fn restore_as_copy(manager: &InstanceManager, id: &str, backup: &str) -> Result<String> {
    crate::utils::fs::safe_file_name(backup)?;
    if !backup.ends_with(".zip") {
        return Err(MonoryxError::UnsafePath(backup.to_string()));
    }
    let backup_path = backup_dir(manager, id)?.join(backup);
    let mut archive = zip::ZipArchive::new(std::fs::File::open(backup_path)?)
        .map_err(|error| MonoryxError::Archive(error.to_string()))?;
    if archive.len() > MAX_FILES + 1 {
        return Err(MonoryxError::Archive(
            "Backup contains too many files".to_string(),
        ));
    }
    let manifest: BackupManifest = {
        let member = archive
            .by_name("monoryx-world.json")
            .map_err(|error| MonoryxError::Archive(error.to_string()))?;
        let mut json = String::new();
        member.take(4096).read_to_string(&mut json)?;
        serde_json::from_str(&json)?
    };
    if manifest.format_version != 1 {
        return Err(MonoryxError::Archive(
            "Unsupported world backup version".to_string(),
        ));
    }
    crate::utils::fs::safe_file_name(&manifest.world_name)?;
    let saves = instance_game(manager, id)?.join("saves");
    std::fs::create_dir_all(&saves)?;
    let temp = saves.join(format!(".monoryx-restore-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&temp)?;
    let result = (|| -> Result<String> {
        let mut total = 0u64;
        for index in 0..archive.len() {
            let mut member = archive
                .by_index(index)
                .map_err(|error| MonoryxError::Archive(error.to_string()))?;
            if member.name() == "monoryx-world.json" {
                continue;
            }
            if member
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                return Err(MonoryxError::Archive(
                    "Linked files are not allowed in world backups".to_string(),
                ));
            }
            let enclosed = member
                .enclosed_name()
                .ok_or_else(|| MonoryxError::Archive("Unsafe path in world backup".to_string()))?;
            let relative = enclosed.strip_prefix("world").map_err(|_| {
                MonoryxError::Archive("Unexpected file in world backup".to_string())
            })?;
            if relative.as_os_str().is_empty() {
                continue;
            }
            if relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(MonoryxError::Archive(
                    "Unsafe path in world backup".to_string(),
                ));
            }
            total = total
                .checked_add(member.size())
                .ok_or_else(|| MonoryxError::Archive("Backup is too large".to_string()))?;
            if total > MAX_BYTES {
                return Err(MonoryxError::Archive("Backup is too large".to_string()));
            }
            let target = temp.join(relative);
            if member.is_dir() {
                std::fs::create_dir_all(target)?;
            } else {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut out = std::fs::File::create(target)?;
                let copied = std::io::copy(&mut member, &mut out)?;
                if copied != member.size() {
                    return Err(MonoryxError::Archive(
                        "Incomplete file in world backup".to_string(),
                    ));
                }
            }
        }
        if !temp.join("level.dat").is_file() {
            return Err(MonoryxError::Archive("Backup has no level.dat".to_string()));
        }
        let name = format!("{} Restored", manifest.world_name);
        let dest = unique_destination(&saves, &name);
        std::fs::rename(&temp, &dest)?;
        Ok(dest
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string())
    })();
    if result.is_err() {
        let _ = crate::utils::fs::remove_dir_inside_root(&saves, &temp);
    }
    result
}

pub fn import_world(manager: &InstanceManager, id: &str, source: &Path) -> Result<String> {
    if !source.join("level.dat").is_file() {
        return Err(MonoryxError::Archive(
            "Choose a world folder containing level.dat".to_string(),
        ));
    }
    let stem = source
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    crate::utils::fs::safe_file_name(&stem)?;
    let saves = instance_game(manager, id)?.join("saves");
    std::fs::create_dir_all(&saves)?;
    let temp = saves.join(format!(".monoryx-import-{}", uuid::Uuid::new_v4()));
    let result = (|| -> Result<String> {
        copy_limited(source, &temp, &mut (0, 0))?;
        let dest = unique_destination(&saves, &stem);
        std::fs::rename(&temp, &dest)?;
        Ok(dest
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string())
    })();
    if result.is_err() {
        let _ = crate::utils::fs::remove_dir_inside_root(&saves, &temp);
    }
    result
}

pub fn import_game_folder(
    manager: &InstanceManager,
    template: &InstanceConfig,
    source: &Path,
) -> Result<InstanceConfig> {
    let game_source = if source.join(".minecraft").is_dir() {
        source.join(".minecraft")
    } else if source.join("game").is_dir() {
        source.join("game")
    } else {
        source.to_path_buf()
    };
    if ![
        "saves",
        "mods",
        "config",
        "resourcepacks",
        "shaderpacks",
        "screenshots",
    ]
    .iter()
    .any(|part| game_source.join(part).is_dir())
        && !game_source.join("options.txt").is_file()
    {
        return Err(MonoryxError::Archive(
            "Choose a Minecraft game folder containing saves, mods, config, or options.txt"
                .to_string(),
        ));
    }
    let name = format!(
        "{} Import",
        source.file_name().unwrap_or_default().to_string_lossy()
    );
    let cfg = manager.create(
        name,
        template.minecraft_version.clone(),
        template.loader,
        template.loader_version.clone(),
    )?;
    let result = (|| -> Result<()> {
        let target = manager.game_dir(&cfg.id);
        let mut budget = (0, 0);
        for folder in [
            "saves",
            "mods",
            "config",
            "resourcepacks",
            "shaderpacks",
            "screenshots",
        ] {
            let src = game_source.join(folder);
            if src.is_dir() {
                copy_limited(&src, &target.join(folder), &mut budget)?;
            }
        }
        for file in ["options.txt", "servers.dat"] {
            let src = game_source.join(file);
            if src.is_file() {
                if src.symlink_metadata()?.file_type().is_symlink() {
                    return Err(MonoryxError::Archive(
                        "Linked files cannot be imported".to_string(),
                    ));
                }
                let size = src.metadata()?.len();
                if budget.0 >= MAX_FILES || budget.1.saturating_add(size) > MAX_BYTES {
                    return Err(MonoryxError::Archive(
                        "Import exceeds the file or size limit".to_string(),
                    ));
                }
                budget.0 += 1;
                budget.1 += size;
                std::fs::copy(src, target.join(file))?;
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        let _ = manager.delete(&cfg.id, true);
        return Err(error);
    }
    Ok(cfg)
}

fn safe_game_path(manager: &InstanceManager, id: &str, relative: &Path) -> Result<PathBuf> {
    if relative
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
        && !relative.as_os_str().is_empty()
    {
        return Err(MonoryxError::UnsafePath(relative.display().to_string()));
    }
    let game = instance_game(manager, id)?;
    let canonical_game = game.canonicalize()?;
    let path = game.join(relative).canonicalize()?;
    if !path.starts_with(canonical_game) {
        return Err(MonoryxError::UnsafePath(relative.display().to_string()));
    }
    Ok(path)
}

pub fn list_files(manager: &InstanceManager, id: &str, relative: &Path) -> Result<Vec<FileEntry>> {
    let dir = safe_game_path(manager, id, relative)?;
    if !dir.is_dir() {
        return Err(MonoryxError::UnsafePath(relative.display().to_string()));
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir)?.take(2000) {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_symlink() || (!ty.is_dir() && !ty.is_file()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let path = relative.join(&name);
        let meta = entry.metadata()?;
        entries.push(FileEntry {
            name,
            relative: path,
            is_dir: ty.is_dir(),
            bytes: meta.len(),
        });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

pub fn preview_text(manager: &InstanceManager, id: &str, relative: &Path) -> Result<String> {
    let path = safe_game_path(manager, id, relative)?;
    let allowed = [
        "txt",
        "log",
        "json",
        "toml",
        "properties",
        "cfg",
        "md",
        "yml",
        "yaml",
        "conf",
    ];
    if !path.is_file()
        || !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| allowed.contains(&extension.to_ascii_lowercase().as_str()))
    {
        return Err(MonoryxError::Archive(
            "Preview is available for text files only".to_string(),
        ));
    }
    if path.metadata()?.len() > PREVIEW_LIMIT {
        return Err(MonoryxError::Archive(
            "File is too large to preview (512 KB limit)".to_string(),
        ));
    }
    String::from_utf8(std::fs::read(path)?)
        .map_err(|_| MonoryxError::Archive("File is not UTF-8 text".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::config::LoaderKind;
    use crate::storage::paths::MonoryxPaths;

    fn setup() -> (tempfile::TempDir, InstanceManager, InstanceConfig) {
        let dir = tempfile::tempdir().unwrap();
        let manager = InstanceManager::new(MonoryxPaths::new(dir.path().to_path_buf()));
        let config = manager
            .create(
                "Test".into(),
                "1.21.1".into(),
                LoaderKind::Fabric,
                "0.16".into(),
            )
            .unwrap();
        (dir, manager, config)
    }

    #[test]
    fn world_backup_and_restore_as_copy() {
        let (_dir, manager, config) = setup();
        let world = manager.game_dir(&config.id).join("saves").join("My World");
        std::fs::create_dir_all(world.join("region")).unwrap();
        std::fs::write(world.join("level.dat"), b"level").unwrap();
        std::fs::write(world.join("region").join("r.0.0.mca"), b"region").unwrap();
        let backup = back_up(&manager, &config.id, "My World").unwrap();
        let restored = restore_as_copy(&manager, &config.id, &backup).unwrap();
        assert_eq!(
            std::fs::read(
                manager
                    .game_dir(&config.id)
                    .join("saves")
                    .join(&restored)
                    .join("level.dat")
            )
            .unwrap(),
            b"level"
        );
        assert_eq!(scan(&manager, &config.id).unwrap().worlds.len(), 2);
        assert!(restore_as_copy(&manager, &config.id, "../other.zip").is_err());
    }

    #[test]
    fn reads_actual_world_name_version_and_size_with_corrupt_metadata_fallback() {
        let (_dir, manager, config) = setup();
        let root = manager.game_dir(&config.id).join("saves/folder-name");
        std::fs::create_dir_all(&root).unwrap();
        let data = b"\x0a\x00\x00\x0a\x00\x04Data\x08\x00\x09LevelName\x00\x09Cozy home\x03\x00\x08GameType\x00\x00\x00\x01\x0a\x00\x07Version\x08\x00\x04Name\x00\x061.21.1\x00\x00\x00";
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gzip.write_all(data).unwrap();
        let bytes = gzip.finish().unwrap();
        std::fs::write(root.join("level.dat"), &bytes).unwrap();
        let snapshot = scan(&manager, &config.id).unwrap();
        let world = &snapshot.worlds[0];
        assert_eq!(world.name, "folder-name");
        assert_eq!(world.display_name, "Cozy home");
        assert_eq!(world.version.as_deref(), Some("1.21.1"));
        assert_eq!(world.mode.as_deref(), Some("Creative"));
        assert_eq!(world.bytes, Some(bytes.len() as u64));
        std::fs::write(root.join("level.dat"), b"broken").unwrap();
        let snapshot = scan(&manager, &config.id).unwrap();
        assert_eq!(snapshot.worlds[0].display_name, "folder-name");
        assert!(snapshot.worlds[0].version.is_none());
    }

    #[test]
    fn file_browser_rejects_escape_and_import_is_separate() {
        let (dir, manager, config) = setup();
        assert!(list_files(&manager, &config.id, Path::new("../outside")).is_err());
        let source = dir.path().join("external");
        std::fs::create_dir_all(source.join(".minecraft").join("saves").join("Old World")).unwrap();
        std::fs::write(
            source
                .join(".minecraft")
                .join("saves")
                .join("Old World")
                .join("level.dat"),
            b"old",
        )
        .unwrap();
        let imported = import_game_folder(&manager, &config, &source).unwrap();
        assert_ne!(imported.id, config.id);
        assert!(manager
            .game_dir(&imported.id)
            .join("saves")
            .join("Old World")
            .join("level.dat")
            .is_file());
    }
}
