use crate::content::{ContentKind, ContentStore};
use crate::downloads::{DownloadJob, DownloadManager};
use crate::error::{MonoryxError, Result};
use crate::instance::manager::InstanceManager;
use crate::modrinth::api::ModrinthClient;
use crate::modrinth::models::{is_compatible, pick_best_version, primary_file, ProjectVersion};
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

#[derive(Debug, Clone, Default)]
pub struct UpdateScan {
    pub updates: Vec<UpdateInfo>,
    pub checked: usize,
    pub modrinth_checked: usize,
    pub curseforge_checked: usize,
    pub untracked: usize,
    pub errors: Vec<String>,
}

fn best_compatible_version<'a>(
    versions: &'a [ProjectVersion],
    kind: ContentKind,
    minecraft_version: &str,
    loader_id: &str,
) -> Option<&'a ProjectVersion> {
    if kind == ContentKind::Mod {
        pick_best_version(versions, minecraft_version, loader_id)
    } else {
        versions
            .iter()
            .filter(|version| version.game_versions.iter().any(|v| v == minecraft_version))
            .max_by(|a, b| a.date_published.cmp(&b.date_published))
    }
}

fn find_file_path(
    manager: &InstanceManager,
    instance_id: &str,
    kind: ContentKind,
    file_name: &str,
) -> Option<std::path::PathBuf> {
    let active = manager
        .game_dir(instance_id)
        .join(kind.subdir())
        .join(file_name);
    if active.is_file() {
        return Some(active);
    }
    let disabled = manager
        .disabled_dir(instance_id)
        .join(kind.subdir())
        .join(format!("{file_name}.disabled"));
    if disabled.is_file() {
        return Some(disabled);
    }
    let legacy = manager
        .disabled_dir(instance_id)
        .join(format!("{file_name}.disabled"));
    if legacy.is_file() {
        return Some(legacy);
    }
    None
}

pub async fn check_updates(
    mr: &ModrinthClient,
    cf: Option<&crate::curseforge::CurseForgeClient>,
    manager: &InstanceManager,
    instance_id: &str,
    minecraft_version: &str,
    loader: &str,
) -> Result<UpdateScan> {
    let store = ContentStore::for_instance(&manager.instance_dir(instance_id));
    let mut content = store.load_result()?;
    let game_dir = manager.game_dir(instance_id);
    let disabled_dir = manager.disabled_dir(instance_id);
    for kind in [
        ContentKind::Mod,
        ContentKind::Resourcepack,
        ContentKind::Shader,
    ] {
        for (folder, enabled) in [
            (game_dir.join(kind.subdir()), true),
            (disabled_dir.join(kind.subdir()), false),
        ] {
            if let Ok(files) = std::fs::read_dir(folder) {
                for file in files.flatten() {
                    if !file.file_type().is_ok_and(|k| k.is_file()) {
                        continue;
                    }
                    let Some(raw) = file.file_name().to_str().map(str::to_string) else {
                        continue;
                    };
                    let name = if enabled {
                        raw.as_str()
                    } else {
                        raw.strip_suffix(".disabled").unwrap_or(&raw)
                    };
                    if !name.ends_with(".jar") && !name.ends_with(".zip") {
                        continue;
                    }
                    if content
                        .entries
                        .iter()
                        .any(|e| e.kind == kind && e.file_name == name)
                    {
                        continue;
                    }
                    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
                    content.entries.push(crate::content::InstalledEntry {
                        file_name: name.to_string(),
                        kind,
                        project_id: None,
                        project_slug: None,
                        project_title: None,
                        version_id: None,
                        version_number: None,
                        file_hash_sha512: None,
                        file_hash_sha1: None,
                        size,
                        enabled,
                        installed_at: String::new(),
                        loader: String::new(),
                        game_version: String::new(),
                    });
                }
            }
        }
    }

    let mut sha1_map: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, entry) in content.entries.iter_mut().enumerate() {
        if entry.project_id.is_some() {
            continue;
        }
        if entry.file_hash_sha1.is_none() {
            if let Some(path) = find_file_path(manager, instance_id, entry.kind, &entry.file_name) {
                if let Ok(h) = crate::utils::hash::sha1_file(&path) {
                    entry.file_hash_sha1 = Some(h);
                }
            }
        }
        if let Some(h) = &entry.file_hash_sha1 {
            sha1_map.entry(h.clone()).or_default().push(idx);
        }
    }

    if !sha1_map.is_empty() {
        let hashes: Vec<String> = sha1_map.keys().cloned().collect();
        if let Ok(matched) = mr.lookup_hashes(&hashes, "sha1").await {
            for (hash, ver) in matched {
                let project = mr.project(&ver.project_id).await.ok();
                let title = project
                    .as_ref()
                    .map(|p| p.title.clone())
                    .unwrap_or_else(|| ver.name.clone());
                let slug = project
                    .as_ref()
                    .map(|p| p.slug.clone())
                    .unwrap_or_else(|| ver.project_id.clone());
                if let Some(indices) = sha1_map.get(&hash) {
                    for &idx in indices {
                        let e = &mut content.entries[idx];
                        e.project_id = Some(ver.project_id.clone());
                        e.project_slug = Some(slug.clone());
                        e.project_title = Some(title.clone());
                        e.version_id = Some(ver.id.clone());
                        e.version_number = Some(ver.version_number.clone());
                        let _ = store.upsert(e.clone());
                    }
                }
            }
        }
    }

    if let Some(cf_client) = cf {
        let mut fp_map: HashMap<u32, Vec<usize>> = HashMap::new();
        for (idx, entry) in content.entries.iter().enumerate() {
            if entry.project_id.is_some() {
                continue;
            }
            if let Some(path) = find_file_path(manager, instance_id, entry.kind, &entry.file_name) {
                if let Ok(fp) = crate::curseforge::curseforge_fingerprint_file(&path) {
                    fp_map.entry(fp).or_default().push(idx);
                }
            }
        }
        if !fp_map.is_empty() {
            let fps: Vec<u32> = fp_map.keys().copied().collect();
            if let Ok(matches) = cf_client.match_fingerprints(&fps).await {
                for m in matches {
                    let pid = crate::curseforge::slug_for(m.id);
                    let project = cf_client.project(m.id).await.ok();
                    let title = project
                        .as_ref()
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| m.file.display_name.clone());
                    let slug = pid.clone();
                    if let Some(fp) = m.file.file_fingerprint.map(|f| f as u32) {
                        if let Some(indices) = fp_map.get(&fp) {
                            for &idx in indices {
                                let e = &mut content.entries[idx];
                                e.project_id = Some(pid.clone());
                                e.project_slug = Some(slug.clone());
                                e.project_title = Some(title.clone());
                                e.version_id = Some(m.file.id.to_string());
                                e.version_number = Some(m.file.file_name.clone());
                                let _ = store.upsert(e.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    let loader_id = match loader.to_lowercase().as_str() {
        "fabric" => "fabric",
        "quilt" => "quilt",
        "forge" => "forge",
        "neoforge" => "neoforge",
        _ => "minecraft",
    };
    let mut scan = UpdateScan::default();
    for entry in content.entries {
        let Some(pid) = entry.project_id.clone() else {
            scan.untracked += 1;
            continue;
        };
        let is_cf = crate::curseforge::is_curseforge_slug(&pid);
        let versions = if is_cf {
            if let (Some(cf_id), Some(cf_client)) = (crate::curseforge::id_from_slug(&pid), cf) {
                match cf_client.files(cf_id, None, None).await {
                    Ok(files) => {
                        scan.curseforge_checked += 1;
                        files
                            .iter()
                            .map(crate::curseforge::to_project_version)
                            .collect()
                    }
                    Err(error) => {
                        scan.errors.push(format!(
                            "{}: {}",
                            entry.project_title.as_deref().unwrap_or(&entry.file_name),
                            error.user_message()
                        ));
                        continue;
                    }
                }
            } else {
                scan.untracked += 1;
                continue;
            }
        } else {
            match mr.project_versions(&pid, None, None).await {
                Ok(v) => {
                    scan.modrinth_checked += 1;
                    v
                }
                Err(error) => {
                    scan.errors.push(format!(
                        "{}: {}",
                        entry.project_title.as_deref().unwrap_or(&entry.file_name),
                        error.user_message()
                    ));
                    continue;
                }
            }
        };
        scan.checked += 1;
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
        let best = best_compatible_version(&compat, entry.kind, minecraft_version, loader_id);
        let Some(best) = best else {
            continue;
        };
        let current = entry.version_id.clone().unwrap_or_default();
        if compat
            .iter()
            .find(|version| version.id == current)
            .is_some_and(|version| {
                if !best.date_published.is_empty() && !version.date_published.is_empty() {
                    best.date_published <= version.date_published
                } else if let (Ok(b_id), Ok(v_id)) =
                    (best.id.parse::<i64>(), version.id.parse::<i64>())
                {
                    b_id <= v_id
                } else {
                    best.id == version.id
                }
            })
        {
            continue;
        }
        if best.id != current {
            let title = entry.project_title.clone().unwrap_or_else(|| pid.clone());
            scan.updates.push(UpdateInfo {
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
    Ok(scan)
}

#[allow(clippy::too_many_arguments)]
pub async fn update_project(
    dm: &DownloadManager,
    mr: &ModrinthClient,
    cf: Option<&crate::curseforge::CurseForgeClient>,
    manager: &InstanceManager,
    instance_id: &str,
    info: &UpdateInfo,
    minecraft_version: &str,
    loader: &str,
) -> Result<String> {
    let versions = if crate::curseforge::is_curseforge_slug(&info.project_id) {
        if let (Some(cf_id), Some(cf_client)) =
            (crate::curseforge::id_from_slug(&info.project_id), cf)
        {
            cf_client
                .files(cf_id, None, None)
                .await?
                .iter()
                .map(crate::curseforge::to_project_version)
                .collect()
        } else {
            Vec::new()
        }
    } else {
        mr.project_versions(&info.project_id, None, None).await?
    };
    let ver = versions
        .iter()
        .find(|v| v.id == info.new_version_id)
        .cloned()
        .or_else(|| {
            best_compatible_version(&versions, info.kind, minecraft_version, loader).cloned()
        })
        .ok_or_else(|| MonoryxError::Modrinth("update version vanished".to_string()))?;
    let file = primary_file(&ver)
        .ok_or_else(|| MonoryxError::Modrinth("update has no downloadable files".to_string()))?;
    let store = ContentStore::for_instance(&manager.instance_dir(instance_id));
    let previous = store
        .load()
        .entries
        .into_iter()
        .find(|entry| entry.kind == info.kind && entry.file_name == info.file_name);
    let dir = match info.kind {
        ContentKind::Mod => manager.mods_dir(instance_id),
        ContentKind::Resourcepack => manager.resourcepacks_dir(instance_id),
        ContentKind::Shader => manager.shaderpacks_dir(instance_id),
    };
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(crate::utils::fs::safe_file_name(&file.filename)?);
    let mut job = DownloadJob::new(&file.filename, &file.url, dest.clone()).with_size(file.size);
    if let Some(h) = file.sha512() {
        job = job.with_sha512(h.to_string());
    } else if let Some(h) = file.sha1() {
        job = job.with_sha1(h.to_string());
    }
    dm.download(&job, None).await?;
    let was_enabled = previous.as_ref().is_none_or(|entry| entry.enabled);
    if !was_enabled {
        let disabled = manager.disabled_dir(instance_id).join(info.kind.subdir());
        std::fs::create_dir_all(&disabled)?;
        let target = disabled.join(format!("{}.disabled", file.filename));
        if target.exists() {
            std::fs::remove_file(&target)?;
        }
        std::fs::rename(&dest, target)?;
    }
    if file.filename != info.file_name {
        crate::utils::fs::safe_file_name(&info.file_name)?;
        let old = dir.join(&info.file_name);
        let old_disabled = manager
            .disabled_dir(instance_id)
            .join(info.kind.subdir())
            .join(format!("{}.disabled", info.file_name));
        let legacy_disabled = manager
            .disabled_dir(instance_id)
            .join(format!("{}.disabled", info.file_name));
        for p in [&old, &old_disabled, &legacy_disabled] {
            if p.exists() && crate::utils::fs::is_within_root(&manager.instance_dir(instance_id), p)
            {
                let _ = std::fs::remove_file(p);
            }
        }
    }
    store.remove(info.kind, &info.file_name)?;
    store.upsert(crate::content::InstalledEntry {
        file_name: file.filename.clone(),
        kind: info.kind,
        project_id: Some(info.project_id.clone()),
        project_slug: previous
            .as_ref()
            .and_then(|entry| entry.project_slug.clone()),
        project_title: Some(info.title.clone()),
        version_id: Some(ver.id.clone()),
        version_number: Some(ver.version_number.clone()),
        file_hash_sha512: file.sha512().map(str::to_string),
        file_hash_sha1: file.sha1().map(str::to_string),
        size: file.size,
        enabled: was_enabled,
        installed_at: chrono::Utc::now().to_rfc3339(),
        loader: loader.to_string(),
        game_version: minecraft_version.to_string(),
    })?;
    Ok(file.filename.clone())
}

#[allow(clippy::too_many_arguments)]
pub async fn update_all(
    dm: &DownloadManager,
    mr: &ModrinthClient,
    cf: Option<&crate::curseforge::CurseForgeClient>,
    manager: &InstanceManager,
    instance_id: &str,
    minecraft_version: &str,
    loader: &str,
    progress: Option<HashMap<String, String>>,
) -> Result<Vec<String>> {
    let updates = check_updates(mr, cf, manager, instance_id, minecraft_version, loader)
        .await?
        .updates;
    let mut done = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();
    for u in updates {
        let _ = &progress;
        match update_project(
            dm,
            mr,
            cf,
            manager,
            instance_id,
            &u,
            minecraft_version,
            loader,
        )
        .await
        {
            Ok(f) => done.push(f),
            Err(e) => failed.push((u.title.clone(), e.to_string())),
        }
    }
    if !failed.is_empty() && done.is_empty() {
        let (title, reason) = &failed[0];
        return Err(MonoryxError::Download(format!(
            "could not update {title}: {reason}"
        )));
    }
    if !failed.is_empty() {
        tracing::warn!(
            "{} of {} updates failed: {}",
            failed.len(),
            failed.len() + done.len(),
            failed
                .iter()
                .map(|(t, r)| format!("{t} ({r})"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_pack_updates_ignore_loader_and_pick_newest_publish_date() {
        let versions: Vec<ProjectVersion> = serde_json::from_value(serde_json::json!([
            {"id":"old","project_id":"p","name":"Old","version_number":"1", "date_published":"2026-01-01T00:00:00Z", "files":[], "dependencies":[], "game_versions":["1.21.1"], "loaders":["minecraft"]},
            {"id":"new","project_id":"p","name":"New","version_number":"2", "date_published":"2026-02-01T00:00:00Z", "files":[], "dependencies":[], "game_versions":["1.21.1"], "loaders":["iris"]}
        ])).unwrap();
        assert_eq!(
            best_compatible_version(&versions, ContentKind::Resourcepack, "1.21.1", "fabric")
                .unwrap()
                .id,
            "new"
        );
    }
}
