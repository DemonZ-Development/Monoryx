pub mod fabric;
pub mod forge;
pub mod neoforge;
pub mod quilt;
pub mod vanilla;

use crate::downloads::DownloadManager;
use crate::error::Result;
use crate::instance::config::LoaderKind;
use crate::minecraft::version::VersionJson;
use crate::storage::paths::MonoryxPaths;
use async_trait::async_trait;

#[async_trait]
pub trait ModLoader: Send + Sync {
    fn id(&self) -> LoaderKind;

    async fn latest_stable(
        &self,
        client: &reqwest::Client,
        minecraft_version: &str,
    ) -> Result<String>;

    async fn available_versions(
        &self,
        client: &reqwest::Client,
        minecraft_version: &str,
    ) -> Result<Vec<String>>;

    async fn install(
        &self,
        dm: &DownloadManager,
        paths: &MonoryxPaths,
        minecraft_version: &str,
        loader_version: &str,
    ) -> Result<VersionJson>;

    async fn is_compatible(
        &self,
        client: &reqwest::Client,
        minecraft_version: &str,
    ) -> Result<bool> {
        Ok(!self
            .available_versions(client, minecraft_version)
            .await?
            .is_empty())
    }
}

#[must_use]
pub fn loader_for(kind: LoaderKind) -> Box<dyn ModLoader> {
    match kind {
        LoaderKind::Vanilla => Box::new(vanilla::VanillaLoader),
        LoaderKind::Fabric => Box::new(fabric::FabricLoader),
        LoaderKind::Quilt => Box::new(quilt::QuiltLoader),
        LoaderKind::Neoforge => Box::new(neoforge::NeoForgeLoader),
        LoaderKind::Forge => Box::new(forge::ForgeLoader),
    }
}

#[derive(serde::Deserialize)]
struct MetaGameVersion {
    version: String,
}

pub(crate) async fn meta_supports_game(
    client: &reqwest::Client,
    meta: &str,
    mc: &str,
) -> Result<bool> {
    let games: Vec<MetaGameVersion> =
        crate::utils::net::get_json_with_retry(client, &format!("{meta}/versions/game"), None)
            .await?;
    Ok(games.iter().any(|game| game.version == mc))
}

pub(crate) fn sort_versions(versions: &mut Vec<String>) {
    fn key(version: &str) -> (Vec<u64>, bool, &str) {
        let (core, suffix) = version.split_once('-').unwrap_or((version, ""));
        (
            core.split('.')
                .map(|part| part.parse().unwrap_or(0))
                .collect(),
            suffix.is_empty(),
            suffix,
        )
    }
    versions.sort_by(
        |a, b| match (semver::Version::parse(a), semver::Version::parse(b)) {
            (Ok(a), Ok(b)) => b.cmp(&a),
            _ => key(b).cmp(&key(a)).then_with(|| b.cmp(a)),
        },
    );
    versions.dedup();
}

pub(crate) fn profile_library_jobs(
    libraries: &[crate::minecraft::version::Library],
    libraries_dir: &std::path::Path,
    default_base: &str,
) -> Result<Vec<crate::downloads::DownloadJob>> {
    libraries
        .iter()
        .filter(|lib| crate::minecraft::libraries::library_applies(lib))
        .map(|lib| {
            let (path, url) = crate::minecraft::libraries::artifact_location(lib, default_base)
                .ok_or_else(|| {
                    crate::error::MonoryxError::LoaderUnavailable(format!(
                        "invalid library coordinate: {}",
                        lib.name
                    ))
                })?;
            let relative = std::path::Path::new(&path);
            if path.contains('\\')
                || path.contains(':')
                || relative
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                return Err(crate::error::MonoryxError::LoaderUnavailable(format!(
                    "invalid library path: {path}"
                )));
            }
            let mut job =
                crate::downloads::DownloadJob::new(&lib.name, url, libraries_dir.join(relative));
            if let Some(artifact) = lib
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.artifact.as_ref())
            {
                job = job.with_sha1(&artifact.sha1).with_size(artifact.size);
            }
            Ok(job)
        })
        .collect()
}

pub(crate) async fn persist_profile(paths: &MonoryxPaths, profile: &VersionJson) -> Result<()> {
    let dir = paths.versions_dir().join(&profile.id);
    std::fs::create_dir_all(&dir)?;
    let text = serde_json::to_string_pretty(profile)?;
    crate::utils::fs::atomic_write(&dir.join(format!("{}.json", profile.id)), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_dependencies_include_asm_mixin_and_remapper_with_rules() {
        let libraries: Vec<crate::minecraft::version::Library> = serde_json::from_value(serde_json::json!([
            {"name":"org.ow2.asm:asm:9.6", "url":"https://maven.fabricmc.net/"},
            {"name":"net.fabricmc:sponge-mixin:0.12.5+mixin.0.8.5", "url":"https://maven.fabricmc.net/"},
            {"name":"net.fabricmc:tiny-remapper:0.10.1", "url":"https://maven.fabricmc.net/"},
            {"name":"org.quiltmc:quilt-loader:0.25.0"},
            {"name":"excluded:library:1", "rules":[{"action":"disallow"}]},
            {"name":"custom:library:1", "downloads":{"artifact":{"path":"custom/library.jar", "url":"https://example.com/library.jar", "sha1":"abc", "size":42}}}
        ])).unwrap();
        let dir = std::path::Path::new("libraries");
        let jobs = profile_library_jobs(
            &libraries,
            dir,
            "https://maven.quiltmc.org/repository/release/",
        )
        .unwrap();
        assert_eq!(jobs.len(), 5);
        assert!(jobs[0]
            .url
            .starts_with("https://maven.fabricmc.net/org/ow2/asm/"));
        assert!(jobs[3]
            .url
            .starts_with("https://maven.quiltmc.org/repository/release/"));
        assert_eq!(jobs[4].dest, dir.join("custom/library.jar"));
        assert_eq!(jobs[4].expected_sha1.as_deref(), Some("abc"));
        assert_eq!(jobs[4].expected_size, Some(42));
    }

    #[test]
    fn profile_dependencies_reject_invalid_coordinates_and_paths() {
        let libraries: Vec<crate::minecraft::version::Library> =
            serde_json::from_value(serde_json::json!([{"name":"bad"}])).unwrap();
        assert!(profile_library_jobs(
            &libraries,
            std::path::Path::new("libraries"),
            "https://example.com/"
        )
        .is_err());
        for path in [
            "../escape.jar",
            "/absolute.jar",
            "C:/escape.jar",
            "..\\escape.jar",
        ] {
            let libraries: Vec<crate::minecraft::version::Library> = serde_json::from_value(serde_json::json!([
                {"name":"a:b:1", "downloads":{"artifact":{"path":path, "url":"https://example.com/a.jar", "sha1":"abc", "size":1}}}
            ])).unwrap();
            assert!(profile_library_jobs(
                &libraries,
                std::path::Path::new("libraries"),
                "https://example.com/"
            )
            .is_err());
        }
    }
}
