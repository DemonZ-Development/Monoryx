use crate::downloads::DownloadManager;
use crate::error::{MonoryxError, Result};
use crate::instance::config::LoaderKind;
use crate::loaders::{persist_profile, ModLoader};
use crate::minecraft::manifest::fetch_manifest;
use crate::minecraft::version::{fetch_version_json, merge_versions, VersionJson};
use crate::storage::paths::MonoryxPaths;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const META: &str = "https://meta.fabricmc.net/v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLoaderVersion {
    pub separator: String,
    pub build: u32,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoaderVersion,
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
struct FabricLoaderVersions(Vec<FabricLoaderEntry>);

impl FabricLoaderVersions {
    fn latest_stable(self, minecraft_version: &str) -> Result<String> {
        self.0
            .into_iter()
            .map(|entry| entry.loader)
            .find(|loader| loader.stable)
            .map(|loader| loader.version)
            .ok_or_else(|| {
                MonoryxError::LoaderUnavailable(format!(
                    "no stable Fabric loader for {minecraft_version}"
                ))
            })
    }

    fn available_versions(self) -> Vec<String> {
        self.0
            .into_iter()
            .map(|entry| entry.loader.version)
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricProfileJson {
    pub id: String,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    #[serde(rename = "time")]
    pub time: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub arguments: Option<crate::minecraft::version::VersionArguments>,
    pub libraries: Vec<crate::minecraft::version::Library>,
}

pub struct FabricLoader;

async fn loader_versions(client: &reqwest::Client, mc: &str) -> Result<FabricLoaderVersions> {
    if !crate::loaders::meta_supports_game(client, META, mc).await? {
        return Ok(FabricLoaderVersions(Vec::new()));
    }
    crate::utils::net::get_json_with_retry(client, &format!("{META}/versions/loader/{mc}"), None)
        .await
}

#[async_trait]
impl ModLoader for FabricLoader {
    fn id(&self) -> LoaderKind {
        LoaderKind::Fabric
    }

    async fn latest_stable(
        &self,
        client: &reqwest::Client,
        minecraft_version: &str,
    ) -> Result<String> {
        let loaders = loader_versions(client, minecraft_version).await?;
        loaders.latest_stable(minecraft_version)
    }

    async fn available_versions(
        &self,
        client: &reqwest::Client,
        minecraft_version: &str,
    ) -> Result<Vec<String>> {
        let loaders = loader_versions(client, minecraft_version).await?;
        Ok(loaders.available_versions())
    }

    async fn install(
        &self,
        dm: &DownloadManager,
        paths: &MonoryxPaths,
        minecraft_version: &str,
        loader_version: &str,
    ) -> Result<VersionJson> {
        let loader_version = if loader_version.trim().is_empty() {
            self.latest_stable(dm.client(), minecraft_version).await?
        } else {
            loader_version.to_string()
        };

        if !self
            .available_versions(dm.client(), minecraft_version)
            .await?
            .contains(&loader_version)
        {
            return Err(MonoryxError::LoaderUnavailable(format!(
                "Fabric {loader_version} is not listed for Minecraft {minecraft_version}"
            )));
        }
        let url =
            format!("{META}/versions/loader/{minecraft_version}/{loader_version}/profile/json");
        let profile: FabricProfileJson =
            crate::utils::net::get_json_with_retry(dm.client(), &url, None)
                .await
                .map_err(|e| {
                    MonoryxError::LoaderUnavailable(format!(
                        "Fabric profile failed: {}",
                        e.user_message()
                    ))
                })?;

        let cache = crate::storage::cache::DiskCache::new(
            paths.manifests_dir(),
            std::time::Duration::from_secs(3600),
        );
        let manifest = fetch_manifest(dm.client(), &cache).await?;
        let entry = manifest
            .versions
            .iter()
            .find(|v| v.id == minecraft_version)
            .ok_or_else(|| {
                MonoryxError::VersionNotFound(format!("Minecraft {minecraft_version} not found"))
            })?;
        let base = fetch_version_json(dm.client(), &cache, minecraft_version, &entry.url).await?;

        let fragment = VersionJson {
            id: profile.id.clone(),
            inherits_from: Some(profile.inherits_from.clone()),
            order: None,
            kind: Some(profile.kind.clone()),
            time: Some(profile.time.clone()),
            release_time: Some(profile.release_time.clone()),
            main_class: Some(profile.main_class.clone()),
            minecraft_arguments: None,
            arguments: profile.arguments.clone(),
            libraries: profile.libraries.clone(),
            asset_index: None,
            assets: None,
            downloads: None,
            logging: None,
            java_version: None,
            jar: None,
            minimum_launcher_version: None,
        };
        let merged = merge_versions(&base, &fragment);

        let jobs = crate::loaders::profile_library_jobs(
            &profile.libraries,
            &paths.libraries_dir(),
            "https://maven.fabricmc.net/",
        )?;
        dm.download_all(&jobs, None).await?;
        persist_profile(paths, &merged).await?;
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires access to the live Fabric metadata service"]
    async fn live_game_specific_metadata_decodes() {
        let client = crate::utils::net::create_client().unwrap();
        let versions = FabricLoader
            .available_versions(&client, "26.3")
            .await
            .unwrap();
        assert!(!versions.is_empty());
        assert!(versions.iter().all(|version| !version.trim().is_empty()));
        let stable = FabricLoader.latest_stable(&client, "26.3").await.unwrap();
        assert!(versions.contains(&stable));
    }

    fn loader_entry(version: &str, stable: bool) -> serde_json::Value {
        serde_json::json!({
            "loader": {
                "separator": ".",
                "build": 1,
                "maven": format!("net.fabricmc:fabric-loader:{version}"),
                "version": version,
                "stable": stable
            },
            "intermediary": {
                "maven": "net.fabricmc:intermediary:1.21.1",
                "version": "1.21.1",
                "stable": true
            },
            "launcherMeta": {
                "version": 2,
                "libraries": {"client": [], "common": [], "server": []},
                "mainClass": {
                    "client": "net.fabricmc.loader.impl.launch.knot.KnotClient",
                    "server": "net.fabricmc.loader.impl.launch.knot.KnotServer"
                }
            }
        })
    }

    fn nested_versions() -> FabricLoaderVersions {
        serde_json::from_value(serde_json::json!([
            loader_entry("0.16.10", false),
            loader_entry("0.16.9", true),
            loader_entry("0.16.8", true)
        ]))
        .unwrap()
    }

    #[test]
    fn nested_schema_lists_loader_versions_in_api_order() {
        assert_eq!(
            nested_versions().available_versions(),
            vec!["0.16.10", "0.16.9", "0.16.8"]
        );
    }

    #[test]
    fn nested_schema_selects_first_stable_loader_not_intermediary() {
        assert_eq!(nested_versions().latest_stable("1.21.1").unwrap(), "0.16.9");
    }

    #[test]
    fn empty_results_have_no_available_versions() {
        let versions: FabricLoaderVersions = serde_json::from_str("[]").unwrap();
        assert!(versions.available_versions().is_empty());
    }

    #[test]
    fn empty_results_report_no_stable_loader() {
        let versions: FabricLoaderVersions = serde_json::from_str("[]").unwrap();
        assert!(matches!(
            versions.latest_stable("1.21.1"),
            Err(MonoryxError::LoaderUnavailable(message))
                if message == "no stable Fabric loader for 1.21.1"
        ));
    }

    #[test]
    fn unstable_only_results_report_no_stable_loader() {
        let versions: FabricLoaderVersions =
            serde_json::from_value(serde_json::json!([loader_entry("0.16.10", false)])).unwrap();
        assert!(matches!(
            versions.latest_stable("1.21.1"),
            Err(MonoryxError::LoaderUnavailable(_))
        ));
    }

    #[test]
    fn flat_loader_schema_is_not_a_game_specific_response() {
        let entry = loader_entry("0.16.9", true);
        assert!(
            serde_json::from_value::<FabricLoaderVersions>(serde_json::json!([entry["loader"]]))
                .is_err()
        );
    }

    #[test]
    fn profile_accepts_official_library_shape_and_optional_arguments() {
        let mut json = serde_json::json!({
            "id": "fabric-loader-0.19.5-1.21.1",
            "inheritsFrom": "1.21.1",
            "releaseTime": "2026-09-16T11:06:00+0000",
            "time": "2026-09-16T11:06:00+0000",
            "type": "release",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "arguments": {"game": [], "jvm": ["-DFabricMcEmu= net.minecraft.client.main.Main "]},
            "libraries": [
                {
                    "name": "org.ow2.asm:asm:9.10.1",
                    "url": "https://maven.fabricmc.net/",
                    "sha1": "ada2141c0cc52ee8f5c48cd5fa4ce0e794f22236",
                    "size": 126151
                },
                {
                    "name": "net.fabricmc:intermediary:1.21.1",
                    "url": "https://maven.fabricmc.net/"
                }
            ]
        });
        let profile: FabricProfileJson = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(profile.inherits_from, "1.21.1");
        assert_eq!(profile.arguments.unwrap().jvm.len(), 1);
        assert_eq!(profile.libraries.len(), 2);
        assert!(profile.libraries[1].downloads.is_none());

        json.as_object_mut().unwrap().remove("arguments");
        let profile: FabricProfileJson = serde_json::from_value(json).unwrap();
        assert!(profile.arguments.is_none());
    }
}
