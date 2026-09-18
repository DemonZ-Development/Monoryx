use crate::error::Result;
use serde::{Deserialize, Serialize};

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
    pub sha1: Option<String>,
    pub compliance_level: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionGroup {
    Release,
    Snapshot,
    Old,
}

impl VersionGroup {
    #[must_use]
    pub fn of(kind: &str) -> Self {
        match kind {
            "release" => Self::Release,
            "snapshot" => Self::Snapshot,
            _ => Self::Old,
        }
    }
}

pub async fn fetch_manifest(
    client: &reqwest::Client,
    cache: &crate::storage::cache::DiskCache,
) -> Result<VersionManifest> {
    const KEY: &str = "mojang-version-manifest-v2";
    if let Some(bytes) = cache.get(KEY) {
        if let Ok(m) = serde_json::from_slice::<VersionManifest>(&bytes) {
            return Ok(m);
        }
    }
    match crate::utils::net::get_json_with_retry::<VersionManifest>(
        client,
        VERSION_MANIFEST_URL,
        None,
    )
    .await
    {
        Ok(m) => {
            if let Ok(bytes) = serde_json::to_vec(&m) {
                let _ = cache.put(KEY, &bytes);
            }
            Ok(m)
        }
        Err(e) => {
            if let Some(bytes) = cache.get_stale(KEY) {
                if let Ok(m) = serde_json::from_slice::<VersionManifest>(&bytes) {
                    tracing::warn!("using stale version manifest (offline?)");
                    return Ok(m);
                }
            }
            Err(e)
        }
    }
}

#[must_use]
pub fn maven_coord_to_path(coord: &str) -> Option<String> {
    let parts: Vec<&str> = coord.split(':').collect();
    if parts.len() < 3 || parts.len() > 4 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let (file_artifact, classifier) = if parts.len() == 4 {
        (
            format!("{artifact}-{version}-{}.jar", parts[3]),
            Some(parts[3]),
        )
    } else {
        (format!("{artifact}-{version}.jar"), None)
    };
    let _ = classifier;
    Some(format!("{group}/{artifact}/{version}/{file_artifact}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_basic() {
        assert_eq!(
            maven_coord_to_path("com.google.guava:guava:31.1-jre"),
            Some("com/google/guava/guava/31.1-jre/guava-31.1-jre.jar".to_string())
        );
    }

    #[test]
    fn maven_with_classifier() {
        assert_eq!(
            maven_coord_to_path("org.lwjgl:lwjgl:3.3.1:natives-windows"),
            Some("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar".to_string())
        );
    }

    #[test]
    fn maven_invalid() {
        assert_eq!(maven_coord_to_path("bad"), None);
        assert_eq!(maven_coord_to_path("a:b"), None);
    }

    #[test]
    fn group_classification() {
        assert_eq!(VersionGroup::of("release"), VersionGroup::Release);
        assert_eq!(VersionGroup::of("snapshot"), VersionGroup::Snapshot);
        assert_eq!(VersionGroup::of("old_beta"), VersionGroup::Old);
    }
}
