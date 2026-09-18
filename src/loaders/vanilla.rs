use crate::downloads::DownloadManager;
use crate::error::{MonoryxError, Result};
use crate::instance::config::LoaderKind;
use crate::loaders::ModLoader;
use crate::minecraft::manifest::fetch_manifest;
use crate::minecraft::version::{fetch_version_json, VersionJson};
use crate::storage::paths::MonoryxPaths;
use async_trait::async_trait;

pub struct VanillaLoader;

fn versions_for_mc(
    manifest: &crate::minecraft::manifest::VersionManifest,
    mc: &str,
) -> Vec<String> {
    if manifest.versions.iter().any(|entry| entry.id == mc) {
        vec![String::new()]
    } else {
        Vec::new()
    }
}

#[async_trait]
impl ModLoader for VanillaLoader {
    fn id(&self) -> LoaderKind {
        LoaderKind::Vanilla
    }

    async fn latest_stable(
        &self,
        client: &reqwest::Client,
        minecraft_version: &str,
    ) -> Result<String> {
        self.available_versions(client, minecraft_version)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| MonoryxError::VersionNotFound(minecraft_version.to_string()))
    }

    async fn available_versions(
        &self,
        client: &reqwest::Client,
        minecraft_version: &str,
    ) -> Result<Vec<String>> {
        let manifest: crate::minecraft::manifest::VersionManifest =
            crate::utils::net::get_json_with_retry(
                client,
                crate::minecraft::manifest::VERSION_MANIFEST_URL,
                None,
            )
            .await?;
        Ok(versions_for_mc(&manifest, minecraft_version))
    }

    async fn install(
        &self,
        dm: &DownloadManager,
        paths: &MonoryxPaths,
        minecraft_version: &str,
        _loader_version: &str,
    ) -> Result<VersionJson> {
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
        fetch_version_json(dm.client(), &cache, minecraft_version, &entry.url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availability_requires_exact_manifest_entry_and_keeps_no_loader_sentinel() {
        let manifest = serde_json::from_value(serde_json::json!({
            "latest": {"release":"1.21.1", "snapshot":"24w33a"},
            "versions": [
                {"id":"1.21.1", "type":"release", "url":"https://example.com/release.json", "time":"", "releaseTime":""},
                {"id":"24w33a", "type":"snapshot", "url":"https://example.com/snapshot.json", "time":"", "releaseTime":""}
            ]
        })).unwrap();
        assert_eq!(versions_for_mc(&manifest, "1.21.1"), [""]);
        assert_eq!(versions_for_mc(&manifest, "24w33a"), [""]);
        assert!(versions_for_mc(&manifest, "1.21").is_empty());
        assert!(versions_for_mc(&manifest, "invalid").is_empty());
    }
}
