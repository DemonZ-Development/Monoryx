use crate::error::{MonoryxError, Result};
use serde::{Deserialize, Serialize};

pub const API_BASE: &str = "https://api.curseforge.com/v1";

pub const GAME_ID: i32 = 432;

pub mod class {
    pub const MODS: i32 = 6;
    pub const RESOURCE_PACKS: i32 = 12;
    pub const SHADERS: i32 = 6552;
    pub const MODPACKS: i32 = 4471;
}

pub mod loader {
    pub const FORGE: i32 = 1;
    pub const FABRIC: i32 = 4;
    pub const QUILT: i32 = 5;
    pub const NEOFORGE: i32 = 6;
}

pub mod sort {
    pub const POPULARITY: i32 = 2;
    pub const LAST_UPDATED: i32 = 3;
    pub const NAME: i32 = 4;
    pub const TOTAL_DOWNLOADS: i32 = 6;
}

#[derive(Debug, Clone)]
pub struct CurseForgeClient {
    http: reqwest::Client,
    ua: String,
    api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub url: String,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfMod {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub download_count: u64,
    #[serde(default)]
    pub logo: Option<Asset>,
    #[serde(default)]
    pub authors: Vec<Author>,
    #[serde(default)]
    pub categories: Vec<Category>,
    #[serde(default)]
    pub class_id: Option<i32>,
    #[serde(default)]
    pub date_released: Option<String>,
    #[serde(default)]
    pub date_modified: Option<String>,
    #[serde(default)]
    pub thumbs_up_count: u64,

    #[serde(default)]
    pub description_html: Option<String>,
    #[serde(default)]
    pub screenshots: Vec<Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDependency {
    pub mod_id: i64,
    #[serde(default)]
    pub relation_type: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfFile {
    pub id: i64,
    pub mod_id: i64,
    pub display_name: String,
    pub file_name: String,

    #[serde(default)]
    pub release_type: i32,
    #[serde(default)]
    pub file_length: u64,
    #[serde(default)]
    pub download_count: u64,

    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub md5: Option<String>,

    #[serde(default)]
    pub hashes: Vec<Hash>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<FileDependency>,
    #[serde(default)]
    pub mod_loader: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hash {
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub algo: i32,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub page_size: u32,
    #[serde(default)]
    pub result_count: u32,
    #[serde(default)]
    pub total_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Deserialize)]
struct DataEnvelope<T> {
    data: T,
}

#[derive(Debug, Clone, Deserialize)]
struct DescriptionEnvelope {
    data: String,
}

#[must_use]
pub fn hash_algo_name(algo: i32) -> &'static str {
    match algo {
        1 => "sha1",
        2 => "md5",
        3 => "sha256",
        4 => "sha512",
        _ => "unknown",
    }
}

#[must_use]
pub fn strongest_hash(file: &CfFile) -> Option<(String, String)> {
    let mut best: Option<(i32, &Hash)> = None;
    for hash in &file.hashes {
        if hash.value.is_empty() {
            continue;
        }
        let rank = match hash.algo {
            4 => 4,
            3 => 3,
            1 => 2,
            _ => continue,
        };
        if best.is_none_or(|(_, current)| rank > current.algo) {
            best = Some((rank, hash));
        }
    }
    let (_, hash) = best?;
    Some((
        hash_algo_name(hash.algo).to_string(),
        hash.value.to_ascii_lowercase(),
    ))
}

#[must_use]
pub fn loader_id(loader: &str) -> Option<i32> {
    match loader.trim().to_ascii_lowercase().as_str() {
        "forge" => Some(loader::FORGE),
        "fabric" => Some(loader::FABRIC),
        "quilt" => Some(loader::QUILT),
        "neoforge" => Some(loader::NEOFORGE),
        _ => None,
    }
}

#[must_use]
pub fn class_id_for(project_type: &str) -> Option<i32> {
    match project_type {
        "mod" => Some(class::MODS),
        "resourcepack" => Some(class::RESOURCE_PACKS),
        "shader" => Some(class::SHADERS),
        "modpack" => Some(class::MODPACKS),
        _ => None,
    }
}

impl CurseForgeClient {
    #[must_use]
    pub fn new(http: reqwest::Client, api_key: &str) -> Self {
        Self {
            http,
            ua: crate::utils::net::modrinth_user_agent(),
            api_key: api_key.trim().to_string(),
        }
    }

    #[must_use]
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path_and_query: &str) -> Result<T> {
        if !self.is_configured() {
            return Err(MonoryxError::Modrinth(
                "CurseForge needs an API key. Add one in Settings.".to_string(),
            ));
        }
        let url = format!("{API_BASE}{path_and_query}");
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let resp = self
                .http
                .get(&url)
                .header(reqwest::header::USER_AGENT, &self.ua)
                .header("x-api-key", &self.api_key)
                .header(reqwest::header::ACCEPT, "application/json")
                .send()
                .await?;
            let status = resp.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS && attempt < 5 {
                let wait = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(attempt as u64);
                tokio::time::sleep(std::time::Duration::from_secs(wait.min(10))).await;
                continue;
            }
            if status.is_server_error() && attempt < 4 {
                tokio::time::sleep(std::time::Duration::from_millis(400 * u64::from(attempt)))
                    .await;
                continue;
            }
            if status == reqwest::StatusCode::FORBIDDEN
                || status == reqwest::StatusCode::UNAUTHORIZED
            {
                return Err(MonoryxError::Modrinth(
                    "CurseForge rejected your API key. Check it in Settings.".to_string(),
                ));
            }
            if !status.is_success() {
                return Err(MonoryxError::Modrinth(format!(
                    "CurseForge returned HTTP {status} for {url}"
                )));
            }
            return Ok(resp.json::<T>().await?);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn search(
        &self,
        query: &str,
        class_id: i32,
        game_version: Option<&str>,
        loader: Option<&str>,
        sort_field: i32,
        index: u32,
        page_size: u32,
    ) -> Result<Paginated<CfMod>> {
        let mut url = format!(
            "/mods/search?gameId={GAME_ID}&classId={class_id}&index={index}&pageSize={page_size}&sortField={sort_field}&sortOrder=desc"
        );
        if !query.trim().is_empty() {
            url.push_str(&format!("&searchFilter={}", encode(query)));
        }
        if let Some(version) = game_version.filter(|v| !v.is_empty()) {
            url.push_str(&format!("&gameVersion={}", encode(version)));
        }
        if let Some(loader) = loader.and_then(loader_id) {
            if game_version.is_some_and(|v| !v.is_empty()) {
                url.push_str(&format!("&modLoaderType={loader}"));
            }
        }
        self.get(&url).await
    }

    pub async fn project(&self, id: i64) -> Result<CfMod> {
        let envelope: DataEnvelope<CfMod> = self.get(&format!("/mods/{id}")).await?;
        Ok(envelope.data)
    }

    pub async fn description(&self, id: i64) -> Result<String> {
        let envelope: DescriptionEnvelope = self.get(&format!("/mods/{id}/description")).await?;
        Ok(envelope.data)
    }

    pub async fn files(
        &self,
        mod_id: i64,
        game_version: Option<&str>,
        loader: Option<&str>,
    ) -> Result<Vec<CfFile>> {
        let mut url = format!("/mods/{mod_id}/files");
        let mut query: Vec<String> = Vec::new();
        if let Some(version) = game_version.filter(|v| !v.is_empty()) {
            query.push(format!("gameVersion={}", encode(version)));
        }
        if let Some(loader) = loader.and_then(loader_id) {
            query.push(format!("modLoaderType={loader}"));
        }
        if !query.is_empty() {
            url.push('?');
            url.push_str(&query.join("&"));
        }
        let page: Paginated<CfFile> = self.get(&url).await?;
        Ok(page.data)
    }
}

pub const SLUG_PREFIX: &str = "curseforge-";

#[must_use]
pub fn id_from_slug(slug: &str) -> Option<i64> {
    slug.strip_prefix(SLUG_PREFIX)?.parse().ok()
}

#[must_use]
pub fn slug_for(id: i64) -> String {
    format!("{SLUG_PREFIX}{id}")
}

#[must_use]
pub fn is_curseforge_slug(slug: &str) -> bool {
    slug.starts_with(SLUG_PREFIX)
}

#[must_use]
pub fn project_type_of(class_id: Option<i32>) -> &'static str {
    match class_id {
        Some(class::MODS) => "mod",
        Some(class::RESOURCE_PACKS) => "resourcepack",
        Some(class::SHADERS) => "shader",
        Some(class::MODPACKS) => "modpack",
        _ => "mod",
    }
}

#[must_use]
pub fn to_search_result(project: &CfMod) -> crate::modrinth::models::SearchResult {
    let categories: Vec<String> = project.categories.iter().map(|c| c.slug.clone()).collect();
    let display: Vec<String> = project.categories.iter().map(|c| c.name.clone()).collect();
    crate::modrinth::models::SearchResult {
        slug: slug_for(project.id),
        title: project.name.clone(),
        description: project.summary.clone(),
        categories,
        client_side: "required".to_string(),
        server_side: "unknown".to_string(),
        project_type: project_type_of(project.class_id).to_string(),
        downloads: project.download_count,
        icon_url: project.logo.as_ref().map(|l| l.url.clone()),
        author: project
            .authors
            .first()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Unknown".to_string()),
        display_categories: display,
        versions: Vec::new(),
        follows: project.thumbs_up_count,
        date_created: project.date_released.clone().unwrap_or_default(),
        date_modified: project.date_modified.clone().unwrap_or_default(),
        latest_version: None,
        license: "CurseForge".to_string(),
        gallery: Vec::new(),
        featured_gallery: None,
    }
}

fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(hashes: Vec<Hash>) -> CfFile {
        CfFile {
            id: 1,
            mod_id: 1,
            display_name: "x".into(),
            file_name: "x.jar".into(),
            release_type: 1,
            file_length: 1,
            download_count: 0,
            download_url: Some("https://edge.forgecdn.net/x.jar".into()),
            md5: Some("abc".into()),
            hashes,
            game_versions: vec!["1.21.1".into()],
            dependencies: vec![],
            mod_loader: Some(loader::FORGE),
        }
    }

    #[test]
    fn strongest_hash_prefers_the_largest_digest() {
        let hashes = vec![
            Hash {
                value: "a1".into(),
                algo: 1,
            },
            Hash {
                value: "b2".into(),
                algo: 3,
            },
            Hash {
                value: "c3".into(),
                algo: 4,
            },
        ];
        let (algo, value) = strongest_hash(&file(hashes)).expect("a hash was published");
        assert_eq!(algo, "sha512");
        assert_eq!(value, "c3");
    }

    #[test]
    fn strongest_hash_accepts_sha1_alone_but_ignores_md5() {
        let sha1 = vec![Hash {
            value: "AABB".into(),
            algo: 1,
        }];
        assert_eq!(
            strongest_hash(&file(sha1)),
            Some(("sha1".to_string(), "aabb".to_string())),
            "value must be lowercased for comparison"
        );
        let md5 = vec![Hash {
            value: "dd".into(),
            algo: 2,
        }];
        assert_eq!(
            strongest_hash(&file(md5)),
            None,
            "md5 is too weak to verify an install with"
        );
    }

    #[test]
    fn strongest_hash_is_none_when_curseforge_publishes_nothing() {
        assert_eq!(strongest_hash(&file(Vec::new())), None);
        assert_eq!(
            strongest_hash(&file(vec![Hash {
                value: String::new(),
                algo: 4
            }])),
            None,
            "an empty value is not a hash"
        );
    }

    #[test]
    fn class_ids_match_curseforge() {
        assert_eq!(class_id_for("mod"), Some(6));
        assert_eq!(class_id_for("resourcepack"), Some(12));
        assert_eq!(class_id_for("shader"), Some(6552));
        assert_eq!(class_id_for("modpack"), Some(4471));
        assert_eq!(class_id_for("nonsense"), None);
    }

    #[test]
    fn loader_ids_match_curseforge() {
        assert_eq!(loader_id("Forge"), Some(1));
        assert_eq!(loader_id("fabric"), Some(4));
        assert_eq!(loader_id(" Quilt "), Some(5));
        assert_eq!(loader_id("neoforge"), Some(6));
        assert_eq!(loader_id("vanilla"), None);
    }

    #[test]
    fn unconfigured_client_refuses_before_any_request() {
        let client = CurseForgeClient::new(reqwest::Client::new(), "   ");
        assert!(!client.is_configured());
    }

    #[test]
    fn encode_escapes_query_separators() {
        assert_eq!(encode("jei & more"), "jei%20%26%20more");
        assert_eq!(encode("1.21.1"), "1.21.1");
        assert_eq!(encode("a/b"), "a%2Fb");
    }

    #[test]
    fn deserialises_a_search_response() {
        let body = r#"{
            "data": [
                {
                    "id": 238222,
                    "name": "Just Enough Items",
                    "slug": "jei",
                    "summary": "Item and recipe viewing",
                    "downloadCount": 300000000,
                    "logo": { "url": "https://media.forgecdn.net/avatar/logo.png" },
                    "authors": [ { "name": "mezz", "url": "https://x" } ],
                    "categories": [ { "name": "Utility", "slug": "utility" } ],
                    "classId": 6,
                    "thumbsUpCount": 900
                }
            ],
            "pagination": { "index": 0, "pageSize": 1, "resultCount": 1, "totalCount": 76403 }
        }"#;
        let parsed: Paginated<CfMod> = serde_json::from_str(body).expect("valid CurseForge body");
        assert_eq!(parsed.data.len(), 1);
        assert_eq!(parsed.data[0].id, 238222);
        assert_eq!(parsed.data[0].authors[0].name, "mezz");
        assert_eq!(parsed.pagination.total_count, 76403);
    }

    #[test]
    fn deserialises_a_file_with_dependencies() {
        let body = r#"{
            "data": {
                "id": 4712,
                "modId": 306612,
                "displayName": "File",
                "fileName": "file.jar",
                "releaseType": 1,
                "fileLength": 2048,
                "downloadCount": 10,
                "downloadUrl": "https://edge.forgecdn.net/files/1/2/file.jar",
                "md5": "deadbeef",
                "hashes": [ { "value": "ABCD", "algo": 1 } ],
                "gameVersions": [ "1.21.1", "Forge" ],
                "dependencies": [ { "modId": 238222, "relationType": 3 } ],
                "modLoader": 1
            }
        }"#;
        let envelope: DataEnvelope<CfFile> =
            serde_json::from_str(body).expect("valid CurseForge file body");
        let parsed = envelope.data;
        assert_eq!(parsed.dependencies[0].mod_id, 238222);
        assert_eq!(parsed.dependencies[0].relation_type, Some(3));
        assert_eq!(
            strongest_hash(&parsed),
            Some(("sha1".to_string(), "abcd".to_string()))
        );
    }

    #[test]
    fn slug_round_trips_the_project_id() {
        let slug = slug_for(238222);
        assert_eq!(slug, "curseforge-238222");
        assert_eq!(id_from_slug(&slug), Some(238222));
        assert!(is_curseforge_slug(&slug));
    }

    #[test]
    fn modrinth_slugs_are_never_mistaken_for_curseforge() {
        for slug in ["sodium", "fabric-api", "iris-shaders", "jei-1-2-3"] {
            assert!(
                !is_curseforge_slug(slug),
                "{slug} must stay a Modrinth slug"
            );
            assert_eq!(id_from_slug(slug), None);
        }
    }

    #[test]
    fn a_malformed_curseforge_slug_yields_no_id() {
        assert_eq!(id_from_slug("curseforge-"), None);
        assert_eq!(id_from_slug("curseforge-abc"), None);
        assert!(is_curseforge_slug("curseforge-abc"));
    }

    #[test]
    fn a_curseforge_hit_becomes_a_renderer_ready_result() {
        let project = CfMod {
            id: 238222,
            name: "Just Enough Items".into(),
            slug: "jei".into(),
            summary: "Item and recipe viewing".into(),
            download_count: 300_000_000,
            logo: Some(Asset {
                url: "https://media.forgecdn.net/logo.png".into(),
                thumbnail_url: None,
            }),
            authors: vec![Author {
                name: "mezz".into(),
                url: None,
            }],
            categories: vec![Category {
                name: "Utility".into(),
                slug: "utility".into(),
            }],
            class_id: Some(class::MODS),
            date_released: Some("2014-01-01T00:00:00Z".into()),
            date_modified: None,
            thumbs_up_count: 900,
            description_html: None,
            screenshots: vec![],
        };
        let hit = to_search_result(&project);
        assert_eq!(hit.slug, "curseforge-238222");
        assert_eq!(hit.title, "Just Enough Items");
        assert_eq!(hit.author, "mezz");
        assert_eq!(hit.downloads, 300_000_000);
        assert_eq!(hit.project_type, "mod");
        assert_eq!(hit.categories, vec!["utility".to_string()]);
        assert_eq!(
            hit.icon_url.as_deref(),
            Some("https://media.forgecdn.net/logo.png")
        );
        assert_eq!(id_from_slug(&hit.slug), Some(238222));
    }

    #[test]
    fn class_ids_map_back_to_project_types() {
        assert_eq!(project_type_of(Some(class::MODS)), "mod");
        assert_eq!(project_type_of(Some(class::RESOURCE_PACKS)), "resourcepack");
        assert_eq!(project_type_of(Some(class::SHADERS)), "shader");
        assert_eq!(project_type_of(Some(class::MODPACKS)), "modpack");
        assert_eq!(project_type_of(None), "mod");
    }

    #[test]
    fn curseforge_settings_default_to_no_key() {
        use crate::config::CurseForgeSettings;
        let settings = CurseForgeSettings::default();
        assert!(settings.api_key.is_empty());
        assert!(
            !settings.is_configured(),
            "an unwired integration must never look ready"
        );
    }
}
