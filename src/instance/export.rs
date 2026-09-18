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
    entry.read_to_string(&mut s).map_err(MonoryxError::Io)?;
    Ok(serde_json::from_str(&s)?)
}
