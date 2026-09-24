use crate::downloads::{DownloadJob, DownloadManager};
use crate::error::{MonoryxError, Result};
use crate::instance::config::LoaderKind;
use crate::loaders::{persist_profile, ModLoader};
use crate::minecraft::manifest::fetch_manifest;
use crate::minecraft::version::{fetch_version_json, merge_versions, Library, VersionJson};
use crate::storage::paths::MonoryxPaths;
use async_trait::async_trait;

const MAVEN_BASE: &str = "https://maven.neoforged.net/releases";
const META_XML: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";

pub struct NeoForgeLoader;

#[async_trait]
impl ModLoader for NeoForgeLoader {
    fn id(&self) -> LoaderKind {
        LoaderKind::Neoforge
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
        if minecraft_version == "1.20.1" {
            let xml = crate::utils::net::get_text_with_retry(
                client,
                "https://maven.neoforged.net/releases/net/neoforged/forge/maven-metadata.xml",
            )
            .await?;
            return Ok(crate::loaders::forge::forge_versions_for_mc(
                &crate::loaders::forge::parse_maven_versions(&xml),
                minecraft_version,
            ));
        }
        if neoforge_prefix(minecraft_version).is_none() {
            return Ok(Vec::new());
        }
        let xml = crate::utils::net::get_text_with_retry(client, META_XML).await?;
        Ok(versions_for_mc(
            crate::loaders::forge::parse_maven_versions(&xml),
            minecraft_version,
        ))
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
                "NeoForge {loader_version} is not listed for Minecraft {minecraft_version}"
            )));
        }
        let (artifact, full) = if minecraft_version == "1.20.1" {
            ("forge", format!("{minecraft_version}-{loader_version}"))
        } else {
            ("neoforge", loader_version.clone())
        };
        let installer_url =
            format!("{MAVEN_BASE}/net/neoforged/{artifact}/{full}/{artifact}-{full}-installer.jar");
        let tmp = paths
            .cache_dir()
            .join(format!("neoforge-{loader_version}-installer.jar"));
        std::fs::create_dir_all(paths.cache_dir())?;
        dm.download(
            &DownloadJob::new("NeoForge installer", &installer_url, tmp.clone()),
            None,
        )
        .await
        .map_err(|e| {
            MonoryxError::LoaderUnavailable(format!(
                "couldn't download NeoForge installer: {}",
                e.user_message()
            ))
        })?;

        let profile = crate::loaders::forge::read_forge_installer(&tmp)?;
        if !profile.processors.is_empty() {
            crate::loaders::forge::run_official_installer(
                dm,
                paths,
                &tmp,
                minecraft_version,
                "NeoForge",
            )
            .await?;
        }
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
        let fragment: VersionJson = serde_json::from_str(&profile.version_json).map_err(|e| {
            MonoryxError::LoaderUnavailable(format!("NeoForge version.json parse failed: {e}"))
        })?;
        let mut merged = merge_versions(&base, &fragment);
        for lib_name in &profile.libraries {
            if merged.libraries.iter().any(|l| &l.name == lib_name) {
                continue;
            }
            merged.libraries.push(Library {
                name: lib_name.clone(),
                rules: None,
                downloads: None,
                natives: None,
                extract: None,
                url: Some(MAVEN_BASE.to_string()),
            });
        }
        let jobs: Vec<DownloadJob> = merged
            .libraries
            .iter()
            .filter_map(|l| {
                crate::minecraft::libraries::artifact_location(l, MAVEN_BASE).map(|(_, url)| {
                    let path = crate::minecraft::manifest::maven_coord_to_path(&l.name)
                        .unwrap_or_else(|| format!("{}.jar", l.name.replace(':', "/")));
                    DownloadJob::new(&l.name, url, paths.libraries_dir().join(&path))
                })
            })
            .collect();
        dm.download_all(&jobs, None).await?;
        persist_profile(paths, &merged).await?;
        Ok(merged)
    }
}

fn neoforge_prefix(mc: &str) -> Option<String> {
    let parts: Vec<&str> = mc.split('.').collect();
    if !(2..=3).contains(&parts.len())
        || parts.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|b| b.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        return None;
    }
    let major: u64 = parts[0].parse().ok()?;
    let minor: u64 = parts[1].parse().ok()?;
    let patch: u64 = parts.get(2).unwrap_or(&"0").parse().ok()?;
    if major == 1 && (minor > 20 || (minor == 20 && patch >= 2)) {
        Some(format!("{minor}.{patch}."))
    } else if major >= 26 {
        Some(format!("{major}.{minor}.{patch}."))
    } else {
        None
    }
}

fn versions_for_mc(all: Vec<String>, mc: &str) -> Vec<String> {
    let Some(prefix) = neoforge_prefix(mc) else {
        return Vec::new();
    };
    let mut versions = all
        .into_iter()
        .filter(|version| {
            version.strip_prefix(&prefix).is_some_and(|build| {
                let number = build.split('-').next().unwrap_or("");
                !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())
            })
        })
        .collect();
    crate::loaders::sort_versions(&mut versions);
    versions
}

fn latest_release(versions: Vec<String>, mc: &str) -> Result<String> {
    versions
        .iter()
        .find(|version| !version.contains('-'))
        .or_else(|| versions.first())
        .cloned()
        .ok_or_else(|| MonoryxError::LoaderUnavailable(format!("no NeoForge for Minecraft {mc}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_mapping() {
        for (mc, prefix) in [
            ("1.21", "21.0."),
            ("1.21.1", "21.1."),
            ("1.20.2", "20.2."),
            ("26.1", "26.1.0."),
            ("26.1.2", "26.1.2."),
        ] {
            assert_eq!(neoforge_prefix(mc).as_deref(), Some(prefix));
        }
        for mc in [
            "1.20.1",
            "1.19.4",
            "invalid",
            "1.21.1-pre1",
            "26.1-snapshot-1",
            "1.21.1.2",
            "1.021.1",
            "",
        ] {
            assert_eq!(neoforge_prefix(mc), None);
        }
    }

    #[test]
    fn selects_exact_patch_and_sorts_numeric_builds() {
        let all = [
            "21.1.9",
            "21.0.100",
            "21.1.100-beta",
            "21.10.1",
            "21.1.10",
            "21.1.10",
            "26.1.0.10-beta",
        ];
        let versions = versions_for_mc(all.map(str::to_string).to_vec(), "1.21.1");
        assert_eq!(versions, ["21.1.100-beta", "21.1.10", "21.1.9"]);
        assert_eq!(latest_release(versions, "1.21.1").unwrap(), "21.1.10");
        for mc in ["26.3", "invalid", "1.21.1-pre1"] {
            assert!(versions_for_mc(all.map(str::to_string).to_vec(), mc).is_empty());
        }
    }

    #[test]
    fn new_version_scheme_does_not_mix_hotfixes_or_old_scheme() {
        let all = ["26.1.0.2-beta", "26.1.1.10", "26.1.0.10-beta", "26.1.99"];
        let versions = versions_for_mc(all.map(str::to_string).to_vec(), "26.1");
        assert_eq!(versions, ["26.1.0.10-beta", "26.1.0.2-beta"]);
        assert_eq!(latest_release(versions, "26.1").unwrap(), "26.1.0.10-beta");
    }
}
