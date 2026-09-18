use crate::error::{MonoryxError, Result};
use crate::minecraft::rules::Rule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(default)]
    pub inherits_from: Option<String>,
    #[serde(default)]
    pub order: Option<i32>,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub release_time: Option<String>,
    #[serde(default)]
    pub main_class: Option<String>,
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub arguments: Option<VersionArguments>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub downloads: Option<HashMap<String, VersionDownload>>,
    #[serde(default)]
    pub logging: Option<serde_json::Value>,
    #[serde(default)]
    pub java_version: Option<JavaVersionMeta>,
    #[serde(default)]
    pub jar: Option<String>,
    #[serde(default)]
    pub minimum_launcher_version: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersionMeta {
    pub component: Option<String>,
    pub major_version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionArguments {
    #[serde(default)]
    pub game: Vec<ArgumentValue>,
    #[serde(default)]
    pub jvm: Vec<ArgumentValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    Plain(String),
    Ruled {
        rules: Vec<Rule>,
        value: ArgumentPayload,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentPayload {
    Single(String),
    Many(Vec<String>),
}

impl ArgumentPayload {
    #[must_use]
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s.clone()],
            Self::Many(v) => v.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub total_size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDownload {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub extract: Option<LibraryExtract>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryExtract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

pub async fn fetch_version_json(
    client: &reqwest::Client,
    cache: &crate::storage::cache::DiskCache,
    version_id: &str,
    manifest_url: &str,
) -> Result<VersionJson> {
    let key = format!("mojang-version-{version_id}");

    if let Some(bytes) = cache.get(&key) {
        if let Ok(v) = serde_json::from_slice::<VersionJson>(&bytes) {
            return Ok(v);
        }
    }
    let v: VersionJson = crate::utils::net::get_json_with_retry(client, manifest_url, None).await?;

    let resolved = Box::pin(resolve_inheritance(client, cache, v)).await?;
    if let Ok(bytes) = serde_json::to_vec(&resolved) {
        let _ = cache.put(&key, &bytes);
    }
    Ok(resolved)
}

async fn resolve_inheritance(
    client: &reqwest::Client,
    cache: &crate::storage::cache::DiskCache,
    mut v: VersionJson,
) -> Result<VersionJson> {
    for _ in 0..8 {
        let Some(base_id) = v.inherits_from.clone() else {
            break;
        };
        let base_key = format!("mojang-version-{base_id}");
        let base: Option<VersionJson> = cache
            .get(&base_key)
            .and_then(|b| serde_json::from_slice(&b).ok());

        let Some(base) = base else {
            return Err(MonoryxError::VersionNotFound(format!(
                "base version '{base_id}' for '{}' is not cached; install the vanilla version first",
                v.id
            )));
        };
        v = merge_versions(&base, &v);

        let _ = client;
    }
    Ok(v)
}

pub fn merge_versions(base: &VersionJson, child: &VersionJson) -> VersionJson {
    let mut merged = base.clone();
    merged.id = child.id.clone();
    if child.main_class.is_some() {
        merged.main_class = child.main_class.clone();
    }
    if child.minecraft_arguments.is_some() {
        merged.minecraft_arguments = child.minecraft_arguments.clone();
    }
    if child.arguments.is_some() {
        if let (Some(b), Some(c)) = (&base.arguments, &child.arguments) {
            merged.arguments = Some(VersionArguments {
                game: [b.game.clone(), c.game.clone()].concat(),
                jvm: [b.jvm.clone(), c.jvm.clone()].concat(),
            });
        } else {
            merged.arguments = child.arguments.clone().or_else(|| base.arguments.clone());
        }
    }

    {
        let mut seen = std::collections::HashSet::new();
        let mut libs = Vec::new();
        for lib in child.libraries.iter().chain(base.libraries.iter()) {
            let parts: Vec<_> = lib.name.split(':').collect();
            let key = (
                parts.first().copied().unwrap_or_default(),
                parts.get(1).copied().unwrap_or_default(),
                parts.get(3).copied(),
            );
            if seen.insert(key) {
                libs.push(lib.clone());
            }
        }
        merged.libraries = libs;
    }
    if child.asset_index.is_some() {
        merged.asset_index = child.asset_index.clone();
    }
    if child.assets.is_some() {
        merged.assets = child.assets.clone();
    }
    if child.downloads.is_some() {
        let mut d = base.downloads.clone().unwrap_or_default();
        if let Some(cd) = &child.downloads {
            d.extend(cd.clone());
        }
        merged.downloads = Some(d);
    }
    if child.logging.is_some() {
        merged.logging = child.logging.clone();
    }
    if child.java_version.is_some() {
        merged.java_version = child.java_version.clone();
    }
    if child.jar.is_some() {
        merged.jar = child.jar.clone();
    }
    merged.inherits_from = None;
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(name: &str) -> Library {
        Library {
            name: name.to_string(),
            rules: None,
            downloads: None,
            natives: None,
            extract: None,
            url: None,
        }
    }

    #[test]
    fn merge_dedups_libraries_child_first() {
        let base = VersionJson {
            id: "1.21".into(),
            inherits_from: None,
            order: None,
            kind: None,
            time: None,
            release_time: None,
            main_class: Some("net.minecraft.client.main.Main".into()),
            minecraft_arguments: None,
            arguments: None,
            libraries: vec![lib("a:b:1"), lib("c:d:1")],
            asset_index: None,
            assets: None,
            downloads: None,
            logging: None,
            java_version: None,
            jar: None,
            minimum_launcher_version: None,
        };
        let child = VersionJson {
            id: "1.21-fabric".into(),
            inherits_from: Some("1.21".into()),
            main_class: Some("net.fabricmc.loader.main.ClientMain".into()),
            libraries: vec![lib("c:d:2"), lib("e:f:1")],
            ..base.clone()
        };
        let m = merge_versions(&base, &child);
        assert_eq!(m.id, "1.21-fabric");
        assert_eq!(
            m.main_class.as_deref(),
            Some("net.fabricmc.loader.main.ClientMain")
        );
        let names: Vec<_> = m.libraries.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, vec!["c:d:2", "e:f:1", "a:b:1"]);
        assert!(m.inherits_from.is_none());
    }

    #[test]
    fn merge_keeps_native_classifiers() {
        let native = lib("org.lwjgl:lwjgl:3.4.3:natives-windows");
        let base = VersionJson {
            id: "1.21".into(),
            inherits_from: None,
            order: None,
            kind: None,
            time: None,
            release_time: None,
            main_class: Some("net.minecraft.client.main.Main".into()),
            minecraft_arguments: None,
            arguments: None,
            libraries: vec![
                lib("org.lwjgl:lwjgl:3.4.3"),
                native,
                lib("org.lwjgl:lwjgl:3.4.3:natives-linux"),
            ],
            asset_index: None,
            assets: None,
            downloads: None,
            logging: None,
            java_version: None,
            jar: None,
            minimum_launcher_version: None,
        };
        let child = VersionJson {
            id: "1.21-fabric".into(),
            inherits_from: Some("1.21".into()),
            main_class: Some("net.fabricmc.loader.main.ClientMain".into()),
            libraries: vec![lib("net.fabricmc:fabric-loader:0.19.5")],
            ..base.clone()
        };
        let m = merge_versions(&base, &child);
        let names: Vec<_> = m.libraries.iter().map(|l| l.name.as_str()).collect();
        assert!(names.contains(&"org.lwjgl:lwjgl:3.4.3:natives-windows"));
    }

    #[test]
    fn parse_minimal_version_json() {
        let v: VersionJson = serde_json::from_value(serde_json::json!({
            "id": "1.21",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": [],
        }))
        .unwrap();
        assert_eq!(v.id, "1.21");
    }
}
