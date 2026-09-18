use crate::content::{ContentKind, ContentStore};
use crate::downloads::{DownloadJob, DownloadManager};
use crate::error::{MonoryxError, Result};
use crate::instance::manager::InstanceManager;
use crate::modrinth::api::ModrinthClient;
use crate::modrinth::models::{is_compatible, pick_best_version, primary_file};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub file_name: String,
    pub kind: ContentKind,
    pub project_id: String,
    pub title: String,
    pub current_version: String,
    pub new_version: String,
    pub new_version_id: String,
}

pub async fn check_updates(
    mr: &ModrinthClient,
    manager: &InstanceManager,
    instance_id: &str,
    minecraft_version: &str,
    loader: &str,
) -> Result<Vec<UpdateInfo>> {
    let store = ContentStore::for_instance(&manager.instance_dir(instance_id));
    let content = store.load();
    let loader_id = match loader.to_lowercase().as_str() {
        "fabric" => "fabric",
        "quilt" => "quilt",
        "forge" => "forge",
        "neoforge" => "neoforge",
        _ => "minecraft",
    };
    let mut out = Vec::new();
    for entry in content.entries {
        let Some(pid) = entry.project_id.clone() else {
            continue;
        };
        let versions = match mr.project_versions(&pid, None, None).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        let compat: Vec<_> = versions
            .into_iter()
            .filter(|v| {
                if entry.kind == ContentKind::Mod {
                    is_compatible(v, minecraft_version, loader_id)
                } else {
                    v.game_versions.iter().any(|g| g == minecraft_version)
                }
            })
            .collect();
        let Some(best) = pick_best_version(&compat, minecraft_version, loader_id) else {
            continue;
        };
        let current = entry.version_id.clone().unwrap_or_default();
        if best.id != current {
            let title = entry.project_title.clone().unwrap_or_else(|| pid.clone());
            out.push(UpdateInfo {
                file_name: entry.file_name.clone(),
                kind: entry.kind,
                project_id: pid,
                title,
                current_version: entry.version_number.clone().unwrap_or_default(),
                new_version: best.version_number.clone(),
                new_version_id: best.id.clone(),
            });
        }
    }
    Ok(out)
}

pub async fn update_project(
    dm: &DownloadManager,
    mr: &ModrinthClient,
    manager: &InstanceManager,
    instance_id: &str,
    info: &UpdateInfo,
    minecraft_version: &str,
    loader: &str,
) -> Result<String> {
    let versions = mr.project_versions(&info.project_id, None, None).await?;
    let ver = versions
        .into_iter()
        .find(|v| v.id == info.new_version_id)
        .ok_or_else(|| MonoryxError::Modrinth("update version vanished".to_string()))?;
    let file = primary_file(&ver)
        .ok_or_else(|| MonoryxError::Modrinth("update has no downloadable files".to_string()))?;
    let dir = match info.kind {
        ContentKind::Mod => manager.mods_dir(instance_id),
        ContentKind::Resourcepack => manager.resourcepacks_dir(instance_id),
        ContentKind::Shader => manager.shaderpacks_dir(instance_id),
    };
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(&file.filename);
    let mut job = DownloadJob::new(&file.filename, &file.url, dest.clone()).with_size(file.size);
    if let Some(h) = file.sha512() {
        job = job.with_sha512(h.to_string());
    } else if let Some(h) = file.sha1() {
        job = job.with_sha1(h.to_string());
    }
    dm.download(&job, None).await?;
    if file.filename != info.file_name {
        let old = dir.join(&info.file_name);
        let old_disabled = manager
            .disabled_dir(instance_id)
            .join(format!("{}.disabled", info.file_name));
        for p in [&old, &old_disabled] {
            if p.exists() && crate::utils::fs::is_within_root(&manager.instance_dir(instance_id), p)
            {
                let _ = std::fs::remove_file(p);
            }
        }
    }
    let store = ContentStore::for_instance(&manager.instance_dir(instance_id));
    store.remove(info.kind, &info.file_name)?;
    store.upsert(crate::content::InstalledEntry {
        file_name: file.filename.clone(),
        kind: info.kind,
        project_id: Some(info.project_id.clone()),
        project_slug: None,
        project_title: Some(info.title.clone()),
        version_id: Some(ver.id.clone()),
        version_number: Some(ver.version_number.clone()),
        file_hash_sha512: file.sha512().map(str::to_string),
        file_hash_sha1: file.sha1().map(str::to_string),
        size: file.size,
        enabled: true,
        installed_at: chrono::Utc::now().to_rfc3339(),
        loader: loader.to_string(),
        game_version: minecraft_version.to_string(),
    })?;
    Ok(file.filename.clone())
}

pub async fn update_all(
    dm: &DownloadManager,
    mr: &ModrinthClient,
    manager: &InstanceManager,
    instance_id: &str,
    minecraft_version: &str,
    loader: &str,
    progress: Option<HashMap<String, String>>,
) -> Result<Vec<String>> {
    let updates = check_updates(mr, manager, instance_id, minecraft_version, loader).await?;
    let mut done = Vec::new();
    for u in updates {
        let _ = &progress;
        let f = update_project(dm, mr, manager, instance_id, &u, minecraft_version, loader).await?;
        done.push(f);
    }
    Ok(done)
}
