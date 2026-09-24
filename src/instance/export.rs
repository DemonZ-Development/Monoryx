use crate::error::{MonoryxError, Result};
use crate::instance::config::InstanceConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceExportManifest {
    pub format_version: u32,
    pub instance: InstanceConfig,
    pub mods: Vec<ContentRef>,
    pub resourcepacks: Vec<ContentRef>,
    pub shaderpacks: Vec<ContentRef>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRef {
    pub file_name: String,
    pub sha512: Option<String>,
    pub sha1: Option<String>,
    pub size: u64,
    pub modrinth_project: Option<String>,
    pub modrinth_version: Option<String>,
}

struct ImportGuard<'a> {
    manager: &'a crate::instance::manager::InstanceManager,
    id: String,
    committed: bool,
}

impl Drop for ImportGuard<'_> {
    fn drop(&mut self) {
        if !self.committed {
            let _ = self.manager.delete(&self.id, true);
        }
    }
}

pub fn export_instance(
    instance_dir: &Path,
    cfg: &InstanceConfig,
    out_zip: &Path,
    include_mods: bool,
    modrinth_meta: &std::collections::HashMap<String, (Option<String>, Option<String>)>,
) -> Result<()> {
    let game = instance_dir.join("game");
    let mut manifest = InstanceExportManifest {
        format_version: 1,
        instance: cfg.clone(),
        mods: Vec::new(),
        resourcepacks: Vec::new(),
        shaderpacks: Vec::new(),
        notes: "MONORYX instance export. Install Minecraft files via launcher metadata on import."
            .to_string(),
    };
    for (subdir, slot) in [
        ("mods", &mut manifest.mods),
        ("resourcepacks", &mut manifest.resourcepacks),
        ("shaderpacks", &mut manifest.shaderpacks),
    ] {
        let dir = game.join(subdir);
        if !dir.exists() {
            continue;
        }
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".jar") && !name.ends_with(".zip") {
                continue;
            }
            let p = entry.path();
            let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            let sha512 = crate::utils::hash::sha512_file(&p).ok();
            let sha1 = crate::utils::hash::sha1_file(&p).ok();
            let (proj, ver) = modrinth_meta.get(&name).cloned().unwrap_or((None, None));
            slot.push(ContentRef {
                file_name: name,
                sha512,
                sha1,
                size,
                modrinth_project: proj,
                modrinth_version: ver,
            });
        }
    }

    let file = std::fs::File::create(out_zip)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("monoryx-instance.json", opts)
        .map_err(|e| MonoryxError::Archive(e.to_string()))?;
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    use std::io::Write as _;
    zip.write_all(manifest_json.as_bytes())
        .map_err(MonoryxError::Io)?;
    if include_mods {
        for subdir in ["mods", "resourcepacks", "shaderpacks"] {
            let dir = game.join(subdir);
            if !dir.exists() {
                continue;
            }
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                if !entry.file_type()?.is_file() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                let arc = format!("files/{subdir}/{name}");
                zip.start_file(arc, opts)
                    .map_err(|e| MonoryxError::Archive(e.to_string()))?;
                let bytes = std::fs::read(entry.path())?;
                zip.write_all(&bytes).map_err(MonoryxError::Io)?;
            }
        }
    }

    zip.finish()
        .map_err(|e| MonoryxError::Archive(e.to_string()))?;
    Ok(())
}

pub fn read_export_manifest(zip_path: &Path) -> Result<InstanceExportManifest> {
    let f = std::fs::File::open(zip_path)?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| MonoryxError::Archive(e.to_string()))?;
    let mut entry = zip
        .by_name("monoryx-instance.json")
        .map_err(|e| MonoryxError::Archive(e.to_string()))?;
    let mut s = String::new();
    use std::io::Read as _;
    if entry.size() > 4_000_000 {
        return Err(MonoryxError::Archive(
            "export manifest is too large".to_string(),
        ));
    }
    entry.read_to_string(&mut s).map_err(MonoryxError::Io)?;
    Ok(serde_json::from_str(&s)?)
}

pub fn import_instance_export(
    manager: &crate::instance::manager::InstanceManager,
    zip_path: &Path,
) -> Result<InstanceConfig> {
    use std::io::Read as _;
    let manifest = read_export_manifest(zip_path)?;
    if manifest.format_version != 1 {
        return Err(MonoryxError::Archive(
            "unsupported MONORYX export version".to_string(),
        ));
    }
    let count = manifest.mods.len() + manifest.resourcepacks.len() + manifest.shaderpacks.len();
    if count > 1000 {
        return Err(MonoryxError::Archive(
            "export contains too many files".to_string(),
        ));
    }
    let mut total = 0u64;
    for reference in manifest
        .mods
        .iter()
        .chain(&manifest.resourcepacks)
        .chain(&manifest.shaderpacks)
    {
        crate::utils::fs::safe_file_name(&reference.file_name)?;
        total = total.saturating_add(reference.size);
    }
    if total > 2_000_000_000 {
        return Err(MonoryxError::Archive(
            "export exceeds the 2 GB import limit".to_string(),
        ));
    }
    let mut cfg = manager.create(
        format!("{} Import", manifest.instance.name),
        manifest.instance.minecraft_version.clone(),
        manifest.instance.loader,
        manifest.instance.loader_version.clone(),
    )?;
    let mut guard = ImportGuard {
        manager,
        id: cfg.id.clone(),
        committed: false,
    };
    cfg.memory_min_mb = manifest.instance.memory_min_mb;
    cfg.memory_max_mb = manifest.instance.memory_max_mb;
    cfg.jvm_args = manifest.instance.jvm_args.clone();
    cfg.game_args = manifest.instance.game_args.clone();
    cfg.width = manifest.instance.width;
    cfg.height = manifest.instance.height;
    cfg.fullscreen = manifest.instance.fullscreen;
    cfg.resolved_version_id.clear();
    manager.save(&cfg)?;
    let file = std::fs::File::open(zip_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| MonoryxError::Archive(e.to_string()))?;
    for (dir, references) in [
        ("mods", &manifest.mods),
        ("resourcepacks", &manifest.resourcepacks),
        ("shaderpacks", &manifest.shaderpacks),
    ] {
        let kind = match dir {
            "mods" => crate::content::ContentKind::Mod,
            "resourcepacks" => crate::content::ContentKind::Resourcepack,
            _ => crate::content::ContentKind::Shader,
        };
        for reference in references {
            let archive_name = format!("files/{dir}/{}", reference.file_name);
            let Ok(mut entry) = archive.by_name(&archive_name) else {
                continue;
            };
            if entry.size() != reference.size || entry.size() > 500_000_000 {
                return Err(MonoryxError::Archive(format!(
                    "invalid size for {}",
                    reference.file_name
                )));
            }
            let destination = manager
                .game_dir(&cfg.id)
                .join(dir)
                .join(&reference.file_name);
            let mut output = std::fs::File::create(&destination)?;
            let copied = std::io::copy(&mut entry.by_ref().take(reference.size + 1), &mut output)?;
            if copied != reference.size {
                return Err(MonoryxError::Archive(format!(
                    "incomplete {}",
                    reference.file_name
                )));
            }
            drop(output);
            if let Some(expected) = &reference.sha512 {
                if !crate::utils::hash::sha512_file(&destination)?.eq_ignore_ascii_case(expected) {
                    return Err(MonoryxError::Archive(format!(
                        "hash mismatch for {}",
                        reference.file_name
                    )));
                }
            } else if let Some(expected) = &reference.sha1 {
                if !crate::utils::hash::sha1_file(&destination)?.eq_ignore_ascii_case(expected) {
                    return Err(MonoryxError::Archive(format!(
                        "hash mismatch for {}",
                        reference.file_name
                    )));
                }
            }
            crate::content::ContentStore::for_instance(&manager.instance_dir(&cfg.id)).upsert(
                crate::content::InstalledEntry {
                    file_name: reference.file_name.clone(),
                    kind,
                    project_id: reference.modrinth_project.clone(),
                    project_slug: None,
                    project_title: None,
                    version_id: reference.modrinth_version.clone(),
                    version_number: None,
                    file_hash_sha512: reference.sha512.clone(),
                    file_hash_sha1: reference.sha1.clone(),
                    size: reference.size,
                    enabled: true,
                    installed_at: chrono::Utc::now().to_rfc3339(),
                    loader: cfg.loader.as_str().to_string(),
                    game_version: cfg.minecraft_version.clone(),
                },
            )?;
        }
    }
    guard.committed = true;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::config::LoaderKind;
    use crate::instance::manager::InstanceManager;
    use crate::storage::paths::MonoryxPaths;

    #[test]
    fn export_import_roundtrip_restores_local_mod() {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().join("data"));
        paths.ensure_all().unwrap();
        let manager = InstanceManager::new(paths);
        let original = manager
            .create(
                "Test".into(),
                "1.21.1".into(),
                LoaderKind::Fabric,
                "0.19.5".into(),
            )
            .unwrap();
        std::fs::write(
            manager.mods_dir(&original.id).join("hello.jar"),
            b"mod bytes",
        )
        .unwrap();
        let export = temp.path().join("instance.zip");
        export_instance(
            &manager.instance_dir(&original.id),
            &original,
            &export,
            true,
            &Default::default(),
        )
        .unwrap();
        let imported = import_instance_export(&manager, &export).unwrap();
        assert_ne!(imported.id, original.id);
        assert_eq!(imported.loader_version, "0.19.5");
        assert_eq!(
            std::fs::read(manager.mods_dir(&imported.id).join("hello.jar")).unwrap(),
            b"mod bytes"
        );
    }
}
