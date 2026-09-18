use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Mod,
    Modpack,
    Resourcepack,
    Shader,
}

impl ProjectType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Modpack => "modpack",
            Self::Resourcepack => "resourcepack",
            Self::Shader => "shader",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "mod" => Some(Self::Mod),
            "modpack" => Some(Self::Modpack),
            "resourcepack" => Some(Self::Resourcepack),
            "shader" => Some(Self::Shader),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub slug: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub client_side: String,
    #[serde(default)]
    pub server_side: String,
    #[serde(default)]
    pub project_type: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub license: Option<LicenseInfo>,
    #[serde(default)]
    pub gallery: Vec<GalleryImage>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalleryImage {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub featured: bool,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub slug: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub client_side: String,
    #[serde(default)]
    pub server_side: String,
    #[serde(default)]
    pub project_type: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub display_categories: Vec<String>,
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(default)]
    pub follows: u64,
    #[serde(default)]
    pub date_created: String,
    #[serde(default)]
    pub date_modified: String,
    #[serde(default)]
    pub latest_version: Option<String>,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub gallery: Vec<String>,
    #[serde(default)]
    pub featured_gallery: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchResult>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectVersion {
    pub id: String,
    pub project_id: String,
    #[serde(default)]
    pub author_id: String,
    #[serde(default)]
    pub featured: bool,
    pub name: String,
    pub version_number: String,
    #[serde(default)]
    pub changelog: Option<String>,
    #[serde(default)]
    pub date_published: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub version_type: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub requested_status: Option<String>,
    pub files: Vec<VersionFile>,
    pub dependencies: Vec<VersionDependency>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    pub hashes: HashMap<String, String>,
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub file_type: Option<String>,
}

impl VersionFile {
    #[must_use]
    pub fn sha512(&self) -> Option<&str> {
        self.hashes.get("sha512").map(String::as_str)
    }
    #[must_use]
    pub fn sha1(&self) -> Option<&str> {
        self.hashes.get("sha1").map(String::as_str)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDependency {
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    pub dependency_type: DependencyType,
}

#[must_use]
pub fn pick_best_version<'a>(
    versions: &'a [ProjectVersion],
    mc: &str,
    loader: &str,
) -> Option<&'a ProjectVersion> {
    let mut cands: Vec<&ProjectVersion> = versions
        .iter()
        .filter(|v| {
            (v.game_versions.iter().any(|g| g == mc))
                && (v.loaders.is_empty()
                    || v.loaders.iter().any(|l| l.eq_ignore_ascii_case(loader)))
        })
        .collect();
    cands.sort_by(|a, b| {
        rank_type(&b.version_type)
            .cmp(&rank_type(&a.version_type))
            .then_with(|| b.date_published.cmp(&a.date_published))
    });
    cands.into_iter().next()
}

fn rank_type(t: &str) -> u8 {
    match t {
        "release" => 3,
        "beta" => 2,
        "alpha" => 1,
        _ => 0,
    }
}

#[must_use]
pub fn primary_file(version: &ProjectVersion) -> Option<&VersionFile> {
    version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
}

#[must_use]
pub fn is_compatible(version: &ProjectVersion, mc: &str, loader: &str) -> bool {
    version.game_versions.iter().any(|g| g == mc)
        && (version.loaders.is_empty()
            || version
                .loaders
                .iter()
                .any(|l| l.eq_ignore_ascii_case(loader)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(id: &str, mc: &str, loader: &str, ty: &str, date: &str) -> ProjectVersion {
        ProjectVersion {
            id: id.into(),
            project_id: "p".into(),
            author_id: String::new(),
            featured: false,
            name: id.into(),
            version_number: id.into(),
            changelog: None,
            date_published: date.into(),
            downloads: 0,
            version_type: ty.into(),
            status: "listed".into(),
            requested_status: None,
            files: vec![VersionFile {
                hashes: Default::default(),
                url: "https://x".into(),
                filename: "a.jar".into(),
                primary: true,
                size: 1,
                file_type: None,
            }],
            dependencies: vec![],
            game_versions: vec![mc.into()],
            loaders: vec![loader.into()],
        }
    }

    #[test]
    fn picks_release_over_newer_beta() {
        let versions = vec![
            v("beta-new", "1.21", "fabric", "beta", "2024-06-01T00:00:00Z"),
            v(
                "rel-old",
                "1.21",
                "fabric",
                "release",
                "2024-01-01T00:00:00Z",
            ),
        ];
        assert_eq!(
            pick_best_version(&versions, "1.21", "fabric").unwrap().id,
            "rel-old"
        );
    }

    #[test]
    fn rejects_wrong_mc_or_loader() {
        let versions = vec![v("a", "1.20.1", "forge", "release", "2024-01-01T00:00:00Z")];
        assert!(pick_best_version(&versions, "1.21", "fabric").is_none());
    }

    #[test]
    fn primary_file_prefers_flag() {
        let mut ver = v("a", "1.21", "fabric", "release", "2024-01-01T00:00:00Z");
        ver.files.push(VersionFile {
            hashes: Default::default(),
            url: "https://y".into(),
            filename: "b.jar".into(),
            primary: false,
            size: 2,
            file_type: None,
        });
        assert_eq!(primary_file(&ver).unwrap().filename, "a.jar");
    }
}
