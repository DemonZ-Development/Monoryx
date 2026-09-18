use crate::content::{ContentKind, ContentStore, InstalledEntry};
use crate::downloads::{DownloadJob, DownloadManager};
use crate::error::{MonoryxError, Result};
use crate::instance::manager::InstanceManager;
use crate::modrinth::api::ModrinthClient;
use crate::modrinth::dependencies::resolve_required;
use crate::modrinth::models::{is_compatible, pick_best_version, primary_file, ProjectVersion};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

pub type InstallProgress = Arc<dyn Fn(String, usize, usize) + Send + Sync>;

pub struct InstallRequest {
    pub project_id: String,
    pub project_slug: String,
    pub project_title: String,
    pub kind: ContentKind,
    pub minecraft_version: String,
    pub loader: String,
    pub version_id: Option<String>,
}

pub struct InstallOutcome {
    pub installed_files: Vec<String>,
    pub warnings: Vec<String>,
}

fn modrinth_loader_id(loader: &str) -> String {
    match loader.to_lowercase().as_str() {
        "fabric" => "fabric".to_string(),
        "quilt" => "quilt".to_string(),
        "forge" => "forge".to_string(),
        "neoforge" => "neoforge".to_string(),
        _ => "minecraft".to_string(),
    }
}

fn target_dir(manager: &InstanceManager, instance_id: &str, kind: ContentKind) -> PathBuf {
    match kind {
        ContentKind::Mod => manager.mods_dir(instance_id),
        ContentKind::Resourcepack => manager.resourcepacks_dir(instance_id),
        ContentKind::Shader => manager.shaderpacks_dir(instance_id),
    }
}

pub async fn install_project(
    dm: &DownloadManager,
    mr: &ModrinthClient,
    manager: &InstanceManager,
    instance_id: &str,
    req: InstallRequest,
    progress: Option<InstallProgress>,
) -> Result<InstallOutcome> {
    let report = |msg: String, a: usize, b: usize| {
        if let Some(cb) = &progress {
            cb(msg, a, b);
        }
    };
    report("Resolving version...".to_string(), 0, 1);
    let cfg = manager.get(instance_id)?;
    let loader_id_outer = modrinth_loader_id(&req.loader);
    let versions = mr.project_versions(&req.project_id, None, None).await?;
    if versions.is_empty() {
        return Err(MonoryxError::Modrinth(
            "no versions found for this project".to_string(),
        ));
    }
    let chosen: ProjectVersion = if let Some(vid) = &req.version_id {
        versions
            .into_iter()
            .find(|v| &v.id == vid)
            .ok_or_else(|| MonoryxError::Modrinth("requested version not found".to_string()))?
    } else {
        let loader_id = loader_id_outer.clone();
        let game_versions = if req.kind == ContentKind::Mod {
            versions
                .iter()
                .filter(|v| is_compatible(v, &req.minecraft_version, &loader_id))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            versions
                .iter()
                .filter(|v| v.game_versions.iter().any(|g| g == &req.minecraft_version))
                .cloned()
                .collect::<Vec<_>>()
        };
        if game_versions.is_empty() {
            return Err(MonoryxError::DependencyConflict(format!(
                "no compatible version for Minecraft {} ({})",
                req.minecraft_version, req.loader
            )));
        }
        pick_best_version(&game_versions, &req.minecraft_version, &loader_id)
            .cloned()
            .unwrap_or(game_versions[0].clone())
    };
    if req.kind == ContentKind::Mod {
        let loader_id = loader_id_outer.clone();
        if !is_compatible(&chosen, &req.minecraft_version, &loader_id) {
            return Err(MonoryxError::DependencyConflict(
                "selected version is not compatible with this instance".to_string(),
            ));
        }
    }
    let store = ContentStore::for_instance(&manager.instance_dir(instance_id));
    let installed_ids: HashSet<String> = store.project_ids().into_iter().collect();
    let mut to_install: Vec<ProjectVersion> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    if req.kind == ContentKind::Mod {
        let id_map = store.id_to_name();
        let incompat =
            crate::modrinth::dependencies::find_incompatibilities(&chosen, &installed_ids, &id_map);
        if !incompat.is_empty() {
            return Err(MonoryxError::DependencyConflict(format!(
                "Conflict detected: {} ({})",
                req.project_title,
                incompat.join("; ")
            )));
        }
        report("Resolving dependencies...".to_string(), 0, 1);
        let loader_for_deps = loader_id_outer.clone();
        let plan = resolve_required(&chosen, &installed_ids, |pid: &str| {
            let mr = mr.clone();
            let pid = pid.to_string();
            let mc = req.minecraft_version.clone();
            let loader_id = loader_for_deps.clone();
            async move {
                let vers = mr.project_versions(&pid, None, None).await?;
                let compat: Vec<ProjectVersion> = vers
                    .into_iter()
                    .filter(|v| is_compatible(v, &mc, &loader_id))
                    .collect();
                Ok(pick_best_version(&compat, &mc, &loader_id).cloned())
            }
        })
        .await?;
        warnings.extend(plan.warnings);
        for dep in plan.ordered {
            if installed_ids.contains(&dep.project_id) {
                continue;
            }
            if to_install.iter().any(|v| v.project_id == dep.project_id) {
                continue;
            }
            to_install.push(dep.version);
        }
    }
    to_install.push(chosen.clone());
    let total = to_install.len();
    let mut installed_files = Vec::new();
    for (i, ver) in to_install.iter().enumerate() {
        report(
            format!("Downloading {} ({}/{})", ver.project_id, i + 1, total),
            i,
            total,
        );
        let file = primary_file(ver).ok_or_else(|| {
            MonoryxError::Modrinth("version has no downloadable files".to_string())
        })?;
        let kind = if ver.project_id == chosen.project_id {
            req.kind
        } else {
            ContentKind::Mod
        };
        let dir = target_dir(manager, instance_id, kind);
        std::fs::create_dir_all(&dir)?;
        let dest = dir.join(&file.filename);
        let mut job =
            DownloadJob::new(&file.filename, &file.url, dest.clone()).with_size(file.size);
        if let Some(sha512) = file.sha512() {
            job = job.with_sha512(sha512.to_string());
        } else if let Some(sha1) = file.sha1() {
            job = job.with_sha1(sha1.to_string());
        }
        dm.download(&job, None).await?;
        let title = if ver.project_id == chosen.project_id {
            Some(req.project_title.clone())
        } else {
            mr.project(&ver.project_id).await.ok().map(|p| p.title)
        };
        let slug = if ver.project_id == chosen.project_id {
            Some(req.project_slug.clone())
        } else {
            None
        };
        store.upsert(InstalledEntry {
            file_name: file.filename.clone(),
            kind,
            project_id: Some(ver.project_id.clone()),
            project_slug: slug,
            project_title: title,
            version_id: Some(ver.id.clone()),
            version_number: Some(ver.version_number.clone()),
            file_hash_sha512: file.sha512().map(str::to_string),
            file_hash_sha1: file.sha1().map(str::to_string),
            size: file.size,
            enabled: true,
            installed_at: chrono::Utc::now().to_rfc3339(),
            loader: req.loader.clone(),
            game_version: req.minecraft_version.clone(),
        })?;
        installed_files.push(file.filename.clone());
        report(format!("Installed {}", file.filename), i + 1, total);
    }
    let _ = cfg;
    Ok(InstallOutcome {
        installed_files,
        warnings,
    })
}

pub fn remove_project_blocking(
    manager: &InstanceManager,
    instance_id: &str,
    kind: ContentKind,
    file_name: &str,
) -> std::result::Result<(), String> {
    let dir = target_dir(manager, instance_id, kind);
    let direct = dir.join(file_name);
    let disabled = manager
        .disabled_dir(instance_id)
        .join(format!("{file_name}.disabled"));
    let in_place = dir.join(format!("{file_name}.disabled"));
    for p in [&direct, &disabled, &in_place] {
        if p.exists() {
            if !crate::utils::fs::is_within_root(&manager.instance_dir(instance_id), p) {
                return Err("refusing to remove a file outside the instance".to_string());
            }
            std::fs::remove_file(p).map_err(|e| e.to_string())?;
        }
    }
    let store = ContentStore::for_instance(&manager.instance_dir(instance_id));
    store
        .remove(kind, file_name)
        .map_err(|e| e.user_message())?;
    let base = file_name.trim_end_matches(".disabled");
    store.remove(kind, base).map_err(|e| e.user_message())?;
    Ok(())
}
