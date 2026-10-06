use crate::content::{ContentKind, InstalledEntry};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ModMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub dependencies: Vec<String>,
    pub provides: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DependencyNode {
    pub entry: InstalledEntry,
    pub mod_id: String,
    pub display_name: String,
    pub dependencies: Vec<String>,
    pub required_by: Vec<String>,
    pub is_library: bool,
    pub missing_dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MissingDependency {
    pub mod_name: String,
    pub raw_dep: String,
    pub display_dep: String,
    pub search_query: String,
}

#[derive(Debug, Clone, Default)]
pub struct DependencyHierarchy {
    pub root_mods: Vec<DependencyNode>,
    pub shared_libraries: Vec<DependencyNode>,
    pub all_missing: Vec<MissingDependency>,
}

fn is_system_dependency(id: &str) -> bool {
    matches!(
        id.to_ascii_lowercase().as_str(),
        "minecraft"
            | "java"
            | "fabricloader"
            | "fabric"
            | "quilt_loader"
            | "forge"
            | "neoforge"
            | "sponge"
    )
}

pub fn extract_jar_metadata(jar_path: &Path) -> Option<ModMetadata> {
    let file = std::fs::File::open(jar_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;

    let fabric_json_str = {
        if let Ok(mut entry) = archive.by_name("fabric.mod.json") {
            let mut s = String::new();
            use std::io::Read as _;
            if entry.read_to_string(&mut s).is_ok() {
                Some(s)
            } else {
                None
            }
        } else {
            None
        }
    };

    if let Some(s) = fabric_json_str {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&s) {
            let id = val
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !id.is_empty() {
                let name = val
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&id)
                    .to_string();
                let version = val
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut dependencies = Vec::new();
                if let Some(deps) = val.get("depends").and_then(|v| v.as_object()) {
                    for key in deps.keys() {
                        if !is_system_dependency(key) {
                            dependencies.push(key.clone());
                        }
                    }
                }

                let mut provides = Vec::new();
                if let Some(p_val) = val.get("provides") {
                    if let Some(arr) = p_val.as_array() {
                        for item in arr {
                            if let Some(p_id) = item.as_str() {
                                provides.push(p_id.to_string());
                            }
                        }
                    } else if let Some(obj) = p_val.as_object() {
                        for p_id in obj.keys() {
                            provides.push(p_id.clone());
                        }
                    }
                }

                let mut nested_names = Vec::new();
                for i in 0..archive.len() {
                    if let Ok(e) = archive.by_index(i) {
                        let n = e.name();
                        if n.starts_with("META-INF/jars/") && n.ends_with(".jar") {
                            nested_names.push(n.to_string());
                        }
                    }
                }

                for nested_name in nested_names {
                    let sub_data = {
                        if let Ok(mut nested_file) = archive.by_name(&nested_name) {
                            let mut buf = Vec::new();
                            use std::io::Read as _;
                            if nested_file.read_to_end(&mut buf).is_ok() {
                                Some(buf)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Some(buf) = sub_data {
                        let cursor = std::io::Cursor::new(buf);
                        if let Ok(mut sub_archive) = zip::ZipArchive::new(cursor) {
                            if let Ok(mut sub_entry) = sub_archive.by_name("fabric.mod.json") {
                                let mut sub_s = String::new();
                                use std::io::Read as _;
                                if sub_entry.read_to_string(&mut sub_s).is_ok() {
                                    if let Ok(sub_val) =
                                        serde_json::from_str::<serde_json::Value>(&sub_s)
                                    {
                                        if let Some(sub_id) =
                                            sub_val.get("id").and_then(|v| v.as_str())
                                        {
                                            if !sub_id.is_empty()
                                                && !provides.contains(&sub_id.to_string())
                                            {
                                                provides.push(sub_id.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                return Some(ModMetadata {
                    id,
                    name,
                    version,
                    dependencies,
                    provides,
                });
            }
        }
    }

    if let Ok(mut entry) = archive.by_name("quilt.mod.json") {
        let mut s = String::new();
        use std::io::Read as _;
        if entry.read_to_string(&mut s).is_ok() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(ql) = val.get("quilt_loader") {
                    let id = ql
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if !id.is_empty() {
                        let name = val
                            .get("metadata")
                            .and_then(|m| m.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or(&id)
                            .to_string();
                        let version = ql
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let mut dependencies = Vec::new();
                        if let Some(deps) = ql.get("depends").and_then(|v| v.as_array()) {
                            for item in deps {
                                if let Some(dep_id) = item.as_str() {
                                    if !is_system_dependency(dep_id) {
                                        dependencies.push(dep_id.to_string());
                                    }
                                } else if let Some(dep_id) = item.get("id").and_then(|v| v.as_str())
                                {
                                    if !is_system_dependency(dep_id) {
                                        dependencies.push(dep_id.to_string());
                                    }
                                }
                            }
                        }
                        let mut provides = Vec::new();
                        if let Some(p_val) = ql.get("provides") {
                            if let Some(arr) = p_val.as_array() {
                                for item in arr {
                                    if let Some(p_id) = item.as_str() {
                                        provides.push(p_id.to_string());
                                    }
                                }
                            }
                        }
                        return Some(ModMetadata {
                            id,
                            name,
                            version,
                            dependencies,
                            provides,
                        });
                    }
                }
            }
        }
    }

    let mut toml_str = None;
    {
        if let Ok(mut entry) = archive.by_name("META-INF/mods.toml") {
            let mut s = String::new();
            use std::io::Read as _;
            if entry.read_to_string(&mut s).is_ok() {
                toml_str = Some(s);
            }
        }
    }
    if toml_str.is_none() {
        if let Ok(mut entry) = archive.by_name("META-INF/neoforge.mods.toml") {
            let mut s = String::new();
            use std::io::Read as _;
            if entry.read_to_string(&mut s).is_ok() {
                toml_str = Some(s);
            }
        }
    }

    if let Some(s) = toml_str {
        let mut id = String::new();
        let mut name = String::new();
        let mut dependencies = Vec::new();
        for line in s.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("modId=") || trimmed.starts_with("modId =") {
                if let Some(val) = trimmed.split('=').nth(1) {
                    let clean = val.trim().trim_matches('"').trim_matches('\'').trim();
                    if id.is_empty() {
                        id = clean.to_string();
                    } else if !is_system_dependency(clean)
                        && !dependencies.contains(&clean.to_string())
                    {
                        dependencies.push(clean.to_string());
                    }
                }
            } else if (trimmed.starts_with("displayName=") || trimmed.starts_with("displayName ="))
                && name.is_empty()
            {
                if let Some(val) = trimmed.split('=').nth(1) {
                    name = val
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .trim()
                        .to_string();
                }
            }
        }
        if !id.is_empty() {
            if name.is_empty() {
                name = id.clone();
            }
            return Some(ModMetadata {
                id,
                name,
                version: String::new(),
                dependencies,
                provides: Vec::new(),
            });
        }
    }

    None
}

fn normalize_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

pub fn resolve_missing_target(dep_id: &str) -> (String, String) {
    let lower = dep_id.to_ascii_lowercase();
    if lower.starts_with("fabric-") || lower.starts_with("fabric_") {
        (format!("Fabric API ({dep_id})"), "fabric-api".to_string())
    } else if lower.starts_with("quilted_") || lower.starts_with("quilt_") {
        (
            format!("Quilt Standard Libraries ({dep_id})"),
            "qsl".to_string(),
        )
    } else if lower.starts_with("cloth-config") {
        (
            format!("Cloth Config ({dep_id})"),
            "cloth-config".to_string(),
        )
    } else if lower.starts_with("architectury") {
        (
            format!("Architectury API ({dep_id})"),
            "architectury-api".to_string(),
        )
    } else {
        (dep_id.to_string(), dep_id.to_string())
    }
}

pub fn build_hierarchy(mods_dir: &Path, entries: &[InstalledEntry]) -> DependencyHierarchy {
    let mod_entries: Vec<&InstalledEntry> = entries
        .iter()
        .filter(|e| e.kind == ContentKind::Mod)
        .collect();

    if mod_entries.is_empty() {
        return DependencyHierarchy::default();
    }

    let mut metadata_map: Vec<Option<ModMetadata>> = Vec::with_capacity(mod_entries.len());
    for entry in &mod_entries {
        let jar_path = mods_dir.join(&entry.file_name);
        metadata_map.push(extract_jar_metadata(&jar_path));
    }

    let mut id_to_index: HashMap<String, usize> = HashMap::new();
    let mut normalized_to_index: HashMap<String, usize> = HashMap::new();

    for (idx, entry) in mod_entries.iter().enumerate() {
        if let Some(meta) = &metadata_map[idx] {
            let key = meta.id.to_ascii_lowercase();
            id_to_index.insert(key.clone(), idx);
            normalized_to_index.insert(normalize_key(&key), idx);
            for p_id in &meta.provides {
                let p_lower = p_id.to_ascii_lowercase();
                id_to_index.insert(p_lower.clone(), idx);
                normalized_to_index.insert(normalize_key(&p_lower), idx);
            }
        }
        if let Some(slug) = &entry.project_slug {
            let key = slug.to_ascii_lowercase();
            id_to_index.insert(key.clone(), idx);
            normalized_to_index.insert(normalize_key(&key), idx);
        }
        let file_stem = entry
            .file_name
            .strip_suffix(".jar")
            .unwrap_or(&entry.file_name)
            .to_ascii_lowercase();
        normalized_to_index.insert(normalize_key(&file_stem), idx);
        if let Some(title) = &entry.project_title {
            normalized_to_index.insert(normalize_key(title), idx);
        }
    }

    let fabric_api_idx = mod_entries.iter().position(|e| {
        let slug = e.project_slug.as_deref().unwrap_or("").to_ascii_lowercase();
        let title = e
            .project_title
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();
        slug == "fabric-api"
            || title.contains("fabric api")
            || e.file_name.to_ascii_lowercase().starts_with("fabric-api")
    });

    let qsl_idx = mod_entries.iter().position(|e| {
        let slug = e.project_slug.as_deref().unwrap_or("").to_ascii_lowercase();
        let title = e
            .project_title
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();
        slug == "qsl"
            || slug == "quilt-standard-libraries"
            || title.contains("quilt standard libraries")
            || e.file_name.to_ascii_lowercase().starts_with("qsl")
    });

    let mut nodes: Vec<DependencyNode> = Vec::with_capacity(mod_entries.len());
    for (idx, entry) in mod_entries.iter().enumerate() {
        let (mod_id, display_name) = if let Some(meta) = &metadata_map[idx] {
            let name = if let Some(title) = &entry.project_title {
                title.clone()
            } else if !meta.name.is_empty() {
                meta.name.clone()
            } else {
                meta.id.clone()
            };
            (meta.id.clone(), name)
        } else {
            let name = entry
                .project_title
                .as_ref()
                .cloned()
                .unwrap_or_else(|| entry.file_name.clone());
            let id = entry
                .project_slug
                .as_ref()
                .cloned()
                .unwrap_or_else(|| entry.file_name.clone());
            (id, name)
        };

        nodes.push(DependencyNode {
            entry: (*entry).clone(),
            mod_id,
            display_name,
            dependencies: Vec::new(),
            required_by: Vec::new(),
            is_library: false,
            missing_dependencies: Vec::new(),
        });
    }

    let mut all_missing: Vec<MissingDependency> = Vec::new();
    for (idx, _entry) in mod_entries.iter().enumerate() {
        let raw_deps = metadata_map[idx]
            .as_ref()
            .map(|m| m.dependencies.clone())
            .unwrap_or_default();

        let parent_name = nodes[idx].display_name.clone();

        for dep in raw_deps {
            let dep_lower = dep.to_ascii_lowercase();
            let dep_norm = normalize_key(&dep_lower);

            let mut target_idx = id_to_index
                .get(&dep_lower)
                .or_else(|| normalized_to_index.get(&dep_norm))
                .copied();

            if target_idx.is_none() {
                if (dep_lower.starts_with("fabric-") || dep_lower.starts_with("fabric_"))
                    && fabric_api_idx.is_some()
                {
                    target_idx = fabric_api_idx;
                } else if (dep_lower.starts_with("quilted_") || dep_lower.starts_with("quilt_"))
                    && qsl_idx.is_some()
                {
                    target_idx = qsl_idx;
                }
            }

            if let Some(target) = target_idx {
                if target != idx {
                    let dep_name = nodes[target].display_name.clone();
                    if !nodes[idx].dependencies.contains(&dep_name) {
                        nodes[idx].dependencies.push(dep_name);
                    }
                    if !nodes[target].required_by.contains(&parent_name) {
                        nodes[target].required_by.push(parent_name.clone());
                        nodes[target].is_library = true;
                    }
                }
            } else if !is_system_dependency(&dep) {
                if !nodes[idx].missing_dependencies.contains(&dep) {
                    nodes[idx].missing_dependencies.push(dep.clone());
                }
                let (display_dep, search_query) = resolve_missing_target(&dep);
                if !all_missing
                    .iter()
                    .any(|m| m.mod_name == parent_name && m.raw_dep == dep)
                {
                    all_missing.push(MissingDependency {
                        mod_name: parent_name.clone(),
                        raw_dep: dep.clone(),
                        display_dep,
                        search_query,
                    });
                }
            }
        }
    }

    for node in &mut nodes {
        let lower = node.mod_id.to_ascii_lowercase();
        if lower.contains("api")
            || lower.contains("lib")
            || lower.contains("library")
            || lower.contains("cloth-config")
            || lower.contains("architectury")
        {
            node.is_library = true;
        }
    }

    let mut root_mods = Vec::new();
    let mut shared_libraries = Vec::new();

    for node in nodes {
        if node.is_library {
            shared_libraries.push(node);
        } else {
            root_mods.push(node);
        }
    }

    root_mods.sort_by_cached_key(|node| node.display_name.to_lowercase());
    shared_libraries.sort_by(|a, b| {
        b.required_by.len().cmp(&a.required_by.len()).then_with(|| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
        })
    });

    DependencyHierarchy {
        root_mods,
        shared_libraries,
        all_missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_dependencies_filter() {
        assert!(is_system_dependency("minecraft"));
        assert!(is_system_dependency("FabricLoader"));
        assert!(is_system_dependency("neoforge"));
        assert!(is_system_dependency("forge"));
        assert!(is_system_dependency("java"));
        assert!(!is_system_dependency("sodium"));
        assert!(!is_system_dependency("cloth-config"));
    }

    #[test]
    fn missing_target_resolution() {
        let (disp, q) = resolve_missing_target("fabric-resource-loader-v0");
        assert_eq!(q, "fabric-api");
        assert!(disp.contains("Fabric API"));

        let (disp_qsl, q_qsl) = resolve_missing_target("quilted_fabric_api");
        assert_eq!(q_qsl, "qsl");
        assert!(disp_qsl.contains("Quilt Standard Libraries"));
    }

    #[test]
    fn hierarchy_categorizes_root_and_libraries() {
        let entry1 = InstalledEntry {
            file_name: "sodium.jar".into(),
            kind: ContentKind::Mod,
            project_id: None,
            project_slug: Some("sodium".into()),
            project_title: Some("Sodium".into()),
            version_id: None,
            version_number: Some("0.5.8".into()),
            file_hash_sha512: None,
            file_hash_sha1: None,
            size: 1024,
            enabled: true,
            installed_at: String::new(),
            loader: "fabric".into(),
            game_version: "1.20.1".into(),
        };
        let entry2 = InstalledEntry {
            file_name: "fabric-api.jar".into(),
            kind: ContentKind::Mod,
            project_id: None,
            project_slug: Some("fabric-api".into()),
            project_title: Some("Fabric API".into()),
            version_id: None,
            version_number: Some("0.92.0".into()),
            file_hash_sha512: None,
            file_hash_sha1: None,
            size: 2048,
            enabled: true,
            installed_at: String::new(),
            loader: "fabric".into(),
            game_version: "1.20.1".into(),
        };
        let hierarchy = build_hierarchy(Path::new("dummy_nonexistent_dir"), &[entry1, entry2]);
        assert_eq!(hierarchy.root_mods.len(), 1);
        assert_eq!(hierarchy.root_mods[0].display_name, "Sodium");
        assert_eq!(hierarchy.shared_libraries.len(), 1);
        assert_eq!(hierarchy.shared_libraries[0].display_name, "Fabric API");
    }
}
