use crate::downloads::{DownloadJob, DownloadManager};
use crate::error::{MonoryxError, Result};
use crate::instance::config::LoaderKind;
use crate::loaders::{persist_profile, ModLoader};
use crate::minecraft::manifest::fetch_manifest;
use crate::minecraft::version::{fetch_version_json, merge_versions, Library, VersionJson};
use crate::storage::paths::MonoryxPaths;
use async_trait::async_trait;

const MAVEN_BASE: &str = "https://maven.minecraftforge.net";
const META_XML: &str =
    "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";

pub struct ForgeLoader;

pub fn parse_maven_versions(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(s) = rest.find("<version>") {
        rest = &rest[s + 9..];
        if let Some(e) = rest.find("</version>") {
            out.push(rest[..e].trim().to_string());
            rest = &rest[e + 10..];
        } else {
            break;
        }
    }
    out
}

#[must_use]
pub fn forge_versions_for_mc(all: &[String], mc: &str) -> Vec<String> {
    let prefix = format!("{mc}-");
    let mut v: Vec<String> = all
        .iter()
        .filter(|s| s.starts_with(&prefix))
        .map(|s| s[prefix.len()..].to_string())
        .collect();
    crate::loaders::sort_versions(&mut v);
    v
}

#[async_trait]
impl ModLoader for ForgeLoader {
    fn id(&self) -> LoaderKind {
        LoaderKind::Forge
    }

    async fn latest_stable(
        &self,
        client: &reqwest::Client,
        minecraft_version: &str,
    ) -> Result<String> {
        let all = self.available_versions(client, minecraft_version).await?;

        all.into_iter().next().ok_or_else(|| {
            MonoryxError::LoaderUnavailable(format!("no Forge for Minecraft {minecraft_version}"))
        })
    }

    async fn available_versions(
        &self,
        client: &reqwest::Client,
        minecraft_version: &str,
    ) -> Result<Vec<String>> {
        let xml = crate::utils::net::get_text_with_retry(client, META_XML).await?;
        let all = parse_maven_versions(&xml);
        Ok(forge_versions_for_mc(&all, minecraft_version))
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
                "Forge {loader_version} is not listed for Minecraft {minecraft_version}"
            )));
        }
        let full = format!("{minecraft_version}-{loader_version}");
        let installer_url =
            format!("{MAVEN_BASE}/net/minecraftforge/forge/{full}/forge-{full}-installer.jar");
        let tmp = paths
            .cache_dir()
            .join(format!("forge-{full}-installer.jar"));
        std::fs::create_dir_all(paths.cache_dir())?;
        dm.download(
            &DownloadJob::new("Forge installer", &installer_url, tmp.clone()),
            None,
        )
        .await
        .map_err(|e| {
            MonoryxError::LoaderUnavailable(format!(
                "couldn't download Forge installer: {}",
                e.user_message()
            ))
        })?;

        let profile = read_forge_installer(&tmp)?;
        if !profile.processors.is_empty() {
            run_official_installer(dm, paths, &tmp, minecraft_version, "Forge").await?;
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
            MonoryxError::LoaderUnavailable(format!("Forge version.json parse failed: {e}"))
        })?;
        let mut merged = merge_versions(&base, &fragment);

        let mut extra_libs: Vec<Library> = Vec::new();
        for lib_name in &profile.libraries {
            if merged.libraries.iter().any(|l| &l.name == lib_name) {
                continue;
            }
            let parts: Vec<_> = lib_name.split(':').collect();
            if parts.len() >= 3 {
                merged.libraries.retain(|lib| {
                    let other: Vec<_> = lib.name.split(':').collect();
                    other.len() < 3
                        || (other[0], other[1], other.get(3)) != (parts[0], parts[1], parts.get(3))
                });
            }
            extra_libs.push(Library {
                name: lib_name.clone(),
                rules: None,
                downloads: None,
                natives: None,
                extract: None,
                url: Some(MAVEN_BASE.to_string()),
            });
        }
        merged.libraries.extend(extra_libs);

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

#[derive(Debug)]
pub(crate) struct ForgeInstallerProfile {
    pub version_json: String,
    pub libraries: Vec<String>,
    pub processors: Vec<serde_json::Value>,
}

pub(crate) fn read_forge_installer(jar: &std::path::Path) -> Result<ForgeInstallerProfile> {
    use std::io::Read as _;
    let f = std::fs::File::open(jar)?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| MonoryxError::Archive(e.to_string()))?;
    let mut version_json = String::new();
    zip.by_name("version.json")
        .map_err(|_| {
            MonoryxError::LoaderUnavailable("Forge installer lacks version.json".to_string())
        })?
        .read_to_string(&mut version_json)
        .map_err(|e| MonoryxError::Archive(e.to_string()))?;

    let mut text = String::new();
    zip.by_name("install_profile.json")
        .map_err(|e| {
            MonoryxError::Archive(format!(
                "installer lacks readable install_profile.json: {e}"
            ))
        })?
        .read_to_string(&mut text)?;
    let (libraries, processors) = parse_install_profile(&text)?;
    Ok(ForgeInstallerProfile {
        version_json,
        libraries,
        processors,
    })
}

fn parse_install_profile(text: &str) -> Result<(Vec<String>, Vec<serde_json::Value>)> {
    #[derive(serde::Deserialize)]
    struct InstallProfile {
        #[serde(default)]
        libraries: Vec<InstallLibrary>,
        #[serde(default)]
        processors: Vec<serde_json::Value>,
    }
    #[derive(serde::Deserialize)]
    struct InstallLibrary {
        name: String,
    }
    let profile: InstallProfile = serde_json::from_str(text)?;
    Ok((
        profile.libraries.into_iter().map(|lib| lib.name).collect(),
        profile.processors,
    ))
}

pub(crate) async fn run_official_installer(
    dm: &DownloadManager,
    paths: &MonoryxPaths,
    installer: &std::path::Path,
    minecraft_version: &str,
    loader: &str,
) -> Result<()> {
    use crate::java::runtime::{required_major_for_version, select_runtime, JavaMode};
    let required = required_major_for_version(minecraft_version, None).unwrap_or(17);
    let found = crate::java::discovery::discover_all(&paths.java_dir()).await;
    let java = if let Some(runtime) = select_runtime(&found, Some(required), &JavaMode::Automatic) {
        runtime.path
    } else {
        crate::java::managed::install_managed(dm, &paths.java_dir(), required, None).await?
    };
    let target = paths.minecraft_dir();
    tokio::fs::create_dir_all(&target).await?;
    let legacy_profile = target.join("launcher_profiles.json");
    let store_profile = target.join("launcher_profiles_microsoft_store.json");
    if !legacy_profile.exists() && !store_profile.exists() {
        tokio::fs::write(&legacy_profile, b"{\"profiles\":{}}\n").await?;
    }
    let mut cmd = tokio::process::Command::new(java);
    cmd.arg("-jar")
        .arg(installer)
        .arg("--installClient")
        .arg(&target)
        .current_dir(&target)
        .kill_on_drop(true);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x0800_0000);
    let output = tokio::time::timeout(std::time::Duration::from_secs(600), cmd.output())
        .await
        .map_err(|_| {
            MonoryxError::LoaderUnavailable(format!(
                "{loader} installer timed out after 10 minutes"
            ))
        })?
        .map_err(|e| {
            MonoryxError::LoaderUnavailable(format!("could not start {loader} installer: {e}"))
        })?;
    let log_path = paths
        .logs_dir()
        .join(format!("{}-installer.log", loader.to_lowercase()));
    tokio::fs::create_dir_all(paths.logs_dir()).await?;
    let mut log = output.stdout;
    log.extend_from_slice(b"\n--- stderr ---\n");
    log.extend_from_slice(&output.stderr);
    tokio::fs::write(&log_path, &log).await?;
    if !output.status.success() {
        let tail = String::from_utf8_lossy(&log);
        let detail = tail
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("no details");
        return Err(MonoryxError::LoaderUnavailable(format!(
            "{loader} installer exited with {}: {detail}. See {}",
            output.status,
            log_path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_unordered_metadata_and_preserves_legacy_suffixes() {
        let all = [
            "1.20.1-47.2.9",
            "1.20.1-47.2.100",
            "1.20.1-47.2.10",
            "1.20.1-47.2.100",
            "1.20-46.0.1",
            "1.7.10-10.13.4.1614-1.7.10",
        ]
        .map(str::to_string);
        assert_eq!(
            forge_versions_for_mc(&all, "1.20.1"),
            ["47.2.100", "47.2.10", "47.2.9"]
        );
        assert_eq!(
            forge_versions_for_mc(&all, "1.7.10"),
            ["10.13.4.1614-1.7.10"]
        );
        assert!(forge_versions_for_mc(&all, "26.3").is_empty());
    }

    #[test]
    fn malformed_install_profiles_do_not_silently_skip_processors() {
        for text in [
            "{",
            "null",
            r#"{"processors":{}}"#,
            r#"{"processors":null}"#,
        ] {
            assert!(parse_install_profile(text).is_err());
        }
        assert_eq!(
            parse_install_profile(r#"{"processors":[{"jar":"a:b:1"}]}"#)
                .unwrap()
                .1
                .len(),
            1
        );
    }

    #[test]
    fn parses_maven_metadata() {
        let xml = "<metadata><versioning><versions><version>1.20.1-47.2.0</version><version>1.20.1-47.1.0</version></versions></versioning></metadata>";
        assert_eq!(parse_maven_versions(xml).len(), 2);
    }

    #[test]
    fn filters_by_mc_newest_first() {
        let all = vec![
            "1.20.1-47.1.0".to_string(),
            "1.20.1-47.2.0".to_string(),
            "1.19.4-45.1.0".to_string(),
        ];
        assert_eq!(
            forge_versions_for_mc(&all, "1.20.1"),
            vec!["47.2.0".to_string(), "47.1.0".to_string()]
        );
    }
}
