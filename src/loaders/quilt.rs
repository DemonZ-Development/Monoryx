use crate::downloads::DownloadManager;
use crate::error::{MonoryxError, Result};
use crate::instance::config::LoaderKind;
use crate::loaders::{persist_profile, ModLoader};
use crate::minecraft::manifest::fetch_manifest;
use crate::minecraft::version::{fetch_version_json, merge_versions, VersionJson};
use crate::storage::paths::MonoryxPaths;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const META: &str = "https://meta.quiltmc.org/v3";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuiltLoaderVersion {
    pub separator: String,
    pub build: u32,
    pub maven: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
struct QuiltLoaderEntry {
    loader: QuiltLoaderVersion,
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
struct QuiltLoaderVersions(Vec<QuiltLoaderEntry>);

impl QuiltLoaderVersions {
    fn available_versions(self) -> Vec<String> {
        let mut versions = self
            .0
            .into_iter()
            .map(|entry| entry.loader.version)
            .collect();
        crate::loaders::sort_versions(&mut versions);
        versions
    }
}

fn latest_release(versions: Vec<String>, mc: &str) -> Result<String> {
    versions
        .into_iter()
        .find(|version| semver::Version::parse(version).is_ok_and(|version| version.pre.is_empty()))
        .ok_or_else(|| MonoryxError::LoaderUnavailable(format!("no stable Quilt loader for {mc}")))
}

pub struct QuiltLoader;

#[async_trait]
impl ModLoader for QuiltLoader {
    fn id(&self) -> LoaderKind {
        LoaderKind::Quilt
    }

    async fn latest_stable(
        &self,
        client: &reqwest::Client,
        minecraft_version: &str,
    ) -> Result<String> {
        let all = self.available_versions(client, minecraft_version).await?;
        latest_release(all, minecraft_version)
    }

    async fn available_versions(
        &self,
        client: &reqwest::Client,
        minecraft_version: &str,
    ) -> Result<Vec<String>> {
        if !crate::loaders::meta_supports_game(client, META, minecraft_version).await? {
            return Ok(Vec::new());
        }
        let url = format!("{META}/versions/loader/{minecraft_version}");
        let loaders: QuiltLoaderVersions =
            crate::utils::net::get_json_with_retry(client, &url, None)
                .await
                .map_err(|e| {
                    let msg = e.user_message();
                    if msg.contains("404") {
                        return MonoryxError::LoaderUnavailable(format!(
                            "Quilt does not support Minecraft {minecraft_version}"
                        ));
                    }
                    MonoryxError::LoaderUnavailable(format!("Quilt metadata failed: {msg}"))
                })?;
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
                "Quilt {loader_version} is not listed for Minecraft {minecraft_version}"
            )));
        }
        let url =
            format!("{META}/versions/loader/{minecraft_version}/{loader_version}/profile/json");
        let profile: serde_json::Value =
            crate::utils::net::get_json_with_retry(dm.client(), &url, None)
                .await
                .map_err(|e| {
                    MonoryxError::LoaderUnavailable(format!(
                        "Quilt profile failed: {}",
                        e.user_message()
                    ))
                })?;

        let fragment: VersionJson = serde_json::from_value(profile).map_err(|e| {
            MonoryxError::LoaderUnavailable(format!("Quilt profile parse failed: {e}"))
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
        let merged = merge_versions(&base, &fragment);

        let jobs = crate::loaders::profile_library_jobs(
            &fragment.libraries,
            &paths.libraries_dir(),
            "https://maven.quiltmc.org/repository/release/",
        )?;
        dm.download_all(&jobs, None).await?;
        persist_profile(paths, &merged).await?;
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(version: &str) -> serde_json::Value {
        serde_json::json!({
            "loader": {"separator": ".", "build": 0, "maven": format!("org.quiltmc:quilt-loader:{version}"), "version": version},
            "hashed": {"version": "1.21.1"},
            "intermediary": {"version": "1.21.1"}
        })
    }

    #[test]
    fn nested_unordered_metadata_selects_latest_release() {
        let versions: QuiltLoaderVersions = serde_json::from_value(serde_json::json!([
            entry("0.26.0-beta.2"),
            entry("0.24.0"),
            entry("0.26.0-beta.10"),
            entry("0.25.0"),
            entry("0.25.0")
        ]))
        .unwrap();
        let versions = versions.available_versions();
        assert_eq!(
            versions,
            ["0.26.0-beta.10", "0.26.0-beta.2", "0.25.0", "0.24.0"]
        );
        assert_eq!(latest_release(versions, "1.21.1").unwrap(), "0.25.0");
    }

    #[test]
    fn empty_and_prerelease_only_metadata_have_no_stable_loader() {
        let versions: QuiltLoaderVersions = serde_json::from_str("[]").unwrap();
        assert!(latest_release(versions.available_versions(), "1.21.1").is_err());
        assert!(latest_release(vec!["0.26.0-beta.1".into()], "1.21.1").is_err());
        assert!(
            serde_json::from_value::<QuiltLoaderVersions>(serde_json::json!([
                entry("0.25.0")["loader"]
            ]))
            .is_err()
        );
    }
}
