use crate::error::{MonoryxError, Result};
use serde::{Deserialize, Serialize};

pub const DIRECT_API_BASE: &str = "https://api.curseforge.com/v1";
pub const DEFAULT_SERVICE_URL: &str = "https://services.demonz.org/curseforge/v1";
pub const API_BASE: &str = DIRECT_API_BASE;

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
    server_url: String,
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
    #[serde(default)]
    pub file_date: Option<String>,
    #[serde(default)]
    pub file_fingerprint: Option<u64>,
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
        Self::with_server(http, api_key, DEFAULT_SERVICE_URL)
    }

    #[must_use]
    pub fn with_server(http: reqwest::Client, api_key: &str, server_url: &str) -> Self {
        let ep = server_url.trim().trim_end_matches('/');
        let base = if ep.is_empty() {
            DEFAULT_SERVICE_URL
        } else {
            ep
        };
        Self {
            http,
            ua: crate::utils::net::modrinth_user_agent(),
            api_key: api_key.trim().to_string(),
            server_url: base.to_string(),
        }
    }

    #[must_use]
    pub fn is_configured(&self) -> bool {
        true
    }

    #[must_use]
    pub fn has_custom_key(&self) -> bool {
        !self.api_key.is_empty()
    }

    pub(crate) fn api_key(&self) -> &str {
        &self.api_key
    }

    pub(crate) fn server_url(&self) -> &str {
        &self.server_url
    }

    #[must_use]
    pub fn effective_url(&self, path_and_query: &str) -> String {
        if self.server_url != DEFAULT_SERVICE_URL {
            format!("{}{path_and_query}", self.server_url)
        } else if self.has_custom_key() {
            format!("{DIRECT_API_BASE}{path_and_query}")
        } else {
            format!("{}{path_and_query}", self.server_url)
        }
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path_and_query: &str) -> Result<T> {
        let has_key = self.has_custom_key();
        let url = self.effective_url(path_and_query);
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let mut req = self
                .http
                .get(&url)
                .header(reqwest::header::USER_AGENT, &self.ua)
                .header(reqwest::header::ACCEPT, "application/json");
            if has_key {
                req = req.header("x-api-key", &self.api_key);
            }
            let resp = match req.send().await {
                Ok(r) => r,
                Err(err) => {
                    if attempt < 3 {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            300 * u64::from(attempt),
                        ))
                        .await;
                        continue;
                    }
                    if err.is_connect() || err.is_timeout() {
                        return Err(MonoryxError::Modrinth(
                            "CurseForge server is offline or unreachable. Check your connection or use Modrinth.".to_string(),
                        ));
                    }
                    return Err(MonoryxError::Http(err));
                }
            };
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
            if status.is_server_error() && attempt < 3 {
                tokio::time::sleep(std::time::Duration::from_millis(300 * u64::from(attempt)))
                    .await;
                continue;
            }
            if status.is_server_error() {
                return Err(MonoryxError::Modrinth(
                    "CurseForge server is temporarily unavailable. Check back soon or switch to Modrinth.".to_string(),
                ));
            }
            if status == reqwest::StatusCode::FORBIDDEN
                || status == reqwest::StatusCode::UNAUTHORIZED
            {
                if self.server_url != DEFAULT_SERVICE_URL {
                    return Err(MonoryxError::Modrinth(
                        "Custom CurseForge proxy authentication rejected. Check proxy configuration in Settings.".to_string(),
                    ));
                }
                if has_key {
                    return Err(MonoryxError::Modrinth(
                        "CurseForge rejected your custom API key. Check it in Settings."
                            .to_string(),
                    ));
                }
                return Err(MonoryxError::Modrinth(
                    "CurseForge server authentication issue. Check back soon or switch to Modrinth.".to_string(),
                ));
            }
            if !status.is_success() {
                return Err(MonoryxError::Modrinth(format!(
                    "CurseForge returned HTTP {status}"
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

    async fn post<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path_and_query: &str,
        body: &B,
    ) -> Result<T> {
        let has_key = self.has_custom_key();
        let url = self.effective_url(path_and_query);
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let mut req = self
                .http
                .post(&url)
                .header(reqwest::header::USER_AGENT, &self.ua)
                .header(reqwest::header::ACCEPT, "application/json")
                .json(body);
            if has_key {
                req = req.header("x-api-key", &self.api_key);
            }
            let resp = match req.send().await {
                Ok(r) => r,
                Err(err) => {
                    if attempt < 3 {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            300 * u64::from(attempt),
                        ))
                        .await;
                        continue;
                    }
                    if err.is_connect() || err.is_timeout() {
                        return Err(MonoryxError::Modrinth(
                            "CurseForge server is offline or unreachable. Check your connection or use Modrinth.".to_string(),
                        ));
                    }
                    return Err(MonoryxError::Http(err));
                }
            };
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
            if status.is_server_error() && attempt < 3 {
                tokio::time::sleep(std::time::Duration::from_millis(300 * u64::from(attempt)))
                    .await;
                continue;
            }
            if status.is_server_error() {
                return Err(MonoryxError::Modrinth(
                    "CurseForge server is temporarily unavailable. Check back soon or switch to Modrinth.".to_string(),
                ));
            }
            if status == reqwest::StatusCode::FORBIDDEN
                || status == reqwest::StatusCode::UNAUTHORIZED
            {
                if self.server_url != DEFAULT_SERVICE_URL {
                    return Err(MonoryxError::Modrinth(
                        "Custom CurseForge proxy authentication rejected. Check proxy configuration in Settings.".to_string(),
                    ));
                }
                if has_key {
                    return Err(MonoryxError::Modrinth(
                        "CurseForge rejected your custom API key. Check it in Settings."
                            .to_string(),
                    ));
                }
                return Err(MonoryxError::Modrinth(
                    "CurseForge server authentication issue. Check back soon or switch to Modrinth.".to_string(),
                ));
            }
            if !status.is_success() {
                return Err(MonoryxError::Modrinth(format!(
                    "CurseForge returned HTTP {status}"
                )));
            }
            return Ok(resp.json::<T>().await?);
        }
    }

    pub async fn match_fingerprints(&self, fingerprints: &[u32]) -> Result<Vec<FingerprintMatch>> {
        if fingerprints.is_empty() {
            return Ok(Vec::new());
        }
        let body = serde_json::json!({ "fingerprints": fingerprints });
        let envelope: DataEnvelope<FingerprintMatchResult> = self
            .post(&format!("/fingerprints/{GAME_ID}"), &body)
            .await?;
        Ok(envelope.data.exact_matches)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintMatchResult {
    #[serde(default)]
    pub exact_matches: Vec<FingerprintMatch>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintMatch {
    pub id: i64,
    pub file: CfFile,
    #[serde(default)]
    pub latest_files: Vec<CfFile>,
}

#[must_use]
pub fn curseforge_fingerprint(bytes: &[u8]) -> u32 {
    let m: u32 = 0x5bd1e995;
    let r: u32 = 24;

    let mut filtered = Vec::with_capacity(bytes.len());
    for &b in bytes {
        if b != 0x9 && b != 0xa && b != 0xd && b != 0x20 {
            filtered.push(b);
        }
    }

    let len = filtered.len();
    let mut h: u32 = 1 ^ (len as u32);

    let mut i = 0;
    while i + 4 <= len {
        let mut k = u32::from_le_bytes([
            filtered[i],
            filtered[i + 1],
            filtered[i + 2],
            filtered[i + 3],
        ]);
        k = k.wrapping_mul(m);
        k ^= k >> r;
        k = k.wrapping_mul(m);

        h = h.wrapping_mul(m);
        h ^= k;
        i += 4;
    }

    let rem = len - i;
    if rem == 3 {
        h ^= (filtered[i + 2] as u32) << 16;
    }
    if rem >= 2 {
        h ^= (filtered[i + 1] as u32) << 8;
    }
    if rem >= 1 {
        h ^= filtered[i] as u32;
        h = h.wrapping_mul(m);
    }

    h ^= h >> 13;
    h = h.wrapping_mul(m);
    h ^= h >> 15;

    h
}

pub fn curseforge_fingerprint_file(path: &std::path::Path) -> Result<u32> {
    let bytes = std::fs::read(path)?;
    Ok(curseforge_fingerprint(&bytes))
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

#[must_use]
pub fn to_project(
    project: &CfMod,
    description: Option<String>,
) -> crate::modrinth::models::Project {
    let categories: Vec<String> = project.categories.iter().map(|c| c.slug.clone()).collect();
    crate::modrinth::models::Project {
        slug: slug_for(project.id),
        title: project.name.clone(),
        description: project.summary.clone(),
        categories,
        client_side: "required".to_string(),
        server_side: "unknown".to_string(),
        project_type: project_type_of(project.class_id).to_string(),
        downloads: project.download_count,
        icon_url: project.logo.as_ref().map(|l| l.url.clone()),
        author: project.authors.first().map(|a| a.name.clone()),
        license: Some(crate::modrinth::models::LicenseInfo {
            id: "curseforge".to_string(),
            name: "CurseForge".to_string(),
            url: None,
        }),
        gallery: project
            .screenshots
            .iter()
            .map(|s| crate::modrinth::models::GalleryImage {
                url: s.url.clone(),
                featured: false,
                title: None,
            })
            .collect(),
        body: description.or_else(|| project.description_html.clone()),
        updated: project.date_modified.clone(),
        versions: Vec::new(),
        loaders: Vec::new(),
        game_versions: Vec::new(),
    }
}

#[must_use]
pub fn to_project_version(file: &CfFile) -> crate::modrinth::models::ProjectVersion {
    let mut hashes = std::collections::HashMap::new();
    for h in &file.hashes {
        if !h.value.is_empty() {
            hashes.insert(
                hash_algo_name(h.algo).to_string(),
                h.value.to_ascii_lowercase(),
            );
        }
    }
    if let Some(md5) = &file.md5 {
        if !md5.is_empty() && !hashes.contains_key("md5") {
            hashes.insert("md5".to_string(), md5.to_ascii_lowercase());
        }
    }

    let url = file.download_url.clone().unwrap_or_else(|| {
        if file.id > 0 && !file.file_name.is_empty() {
            format!(
                "https://edge.forgecdn.net/files/{}/{}/{}",
                file.id / 1000,
                file.id % 1000,
                file.file_name
            )
        } else {
            String::new()
        }
    });
    let version_type = match file.release_type {
        1 => "release",
        2 => "beta",
        3 => "alpha",
        _ => "release",
    }
    .to_string();

    let mut loaders = match file.mod_loader {
        Some(loader::FORGE) => vec!["forge".to_string()],
        Some(loader::FABRIC) => vec!["fabric".to_string()],
        Some(loader::QUILT) => vec!["quilt".to_string()],
        Some(loader::NEOFORGE) => vec!["neoforge".to_string()],
        _ => Vec::new(),
    };
    if loaders.is_empty() {
        for gv in &file.game_versions {
            let lower = gv.to_ascii_lowercase();
            match lower.as_str() {
                "forge" | "fabric" | "quilt" | "neoforge" if !loaders.contains(&lower) => {
                    loaders.push(lower);
                }
                _ => {}
            }
        }
    }

    let mc_versions: Vec<String> = file
        .game_versions
        .iter()
        .filter(|gv| gv.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .cloned()
        .collect();
    let effective_versions = if mc_versions.is_empty() {
        file.game_versions.clone()
    } else {
        mc_versions
    };

    crate::modrinth::models::ProjectVersion {
        id: file.id.to_string(),
        project_id: slug_for(file.mod_id),
        author_id: String::new(),
        featured: false,
        name: file.display_name.clone(),
        version_number: file.file_name.clone(),
        changelog: None,
        date_published: file
            .file_date
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("{:014}", file.id)),
        downloads: file.download_count,
        version_type,
        status: "listed".to_string(),
        requested_status: None,
        files: vec![crate::modrinth::models::VersionFile {
            hashes,
            url,
            filename: file.file_name.clone(),
            primary: true,
            size: file.file_length,
            file_type: None,
        }],
        dependencies: file
            .dependencies
            .iter()
            .filter_map(|dep| {
                use crate::modrinth::models::{DependencyType, VersionDependency};
                let dependency_type = match dep.relation_type {
                    Some(1 | 6) => DependencyType::Embedded,
                    Some(2) => DependencyType::Optional,
                    Some(3) => DependencyType::Required,
                    Some(5) => DependencyType::Incompatible,
                    _ => return None,
                };
                Some(VersionDependency {
                    project_id: Some(slug_for(dep.mod_id)),
                    version_id: None,
                    file_name: None,
                    dependency_type,
                })
            })
            .collect(),
        game_versions: effective_versions,
        loaders,
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

    #[test]
    fn dependencies_keep_provider_and_relation_type() {
        use crate::modrinth::models::DependencyType;
        let mut input = file(vec![]);
        input.dependencies = [1, 2, 3, 4, 5, 6]
            .into_iter()
            .map(|relation| FileDependency {
                mod_id: 42,
                relation_type: Some(relation),
            })
            .collect();
        let version = to_project_version(&input);
        assert_eq!(version.dependencies.len(), 5);
        assert!(version
            .dependencies
            .iter()
            .all(|dep| dep.project_id.as_deref() == Some("curseforge-42")));
        assert!(version
            .dependencies
            .iter()
            .any(|dep| dep.dependency_type == DependencyType::Required));
        assert!(version
            .dependencies
            .iter()
            .any(|dep| dep.dependency_type == DependencyType::Incompatible));
    }

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
            file_date: None,
            file_fingerprint: None,
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
    fn client_tracks_custom_key_and_defaults_to_server() {
        let client = CurseForgeClient::new(reqwest::Client::new(), "   ");
        assert!(!client.has_custom_key());
        assert!(client.is_configured());
        let custom = CurseForgeClient::new(reqwest::Client::new(), "my-key");
        assert!(custom.has_custom_key());
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
        assert!(!settings.has_custom_key());
        assert!(settings.is_configured());
    }

    #[test]
    fn curseforge_to_project_and_version() {
        let project = CfMod {
            id: 238222,
            name: "JEI".into(),
            slug: "jei".into(),
            summary: "Item viewer".into(),
            download_count: 500,
            logo: None,
            authors: vec![Author {
                name: "mezz".into(),
                url: None,
            }],
            categories: vec![Category {
                name: "Utility".into(),
                slug: "utility".into(),
            }],
            class_id: Some(class::MODS),
            date_released: None,
            date_modified: None,
            thumbs_up_count: 10,
            description_html: None,
            screenshots: vec![],
        };
        let p = to_project(&project, Some("<p>Hello</p>".into()));
        assert_eq!(p.slug, "curseforge-238222");
        assert_eq!(p.title, "JEI");
        assert_eq!(p.body.as_deref(), Some("<p>Hello</p>"));

        let file = CfFile {
            id: 12345,
            mod_id: 238222,
            display_name: "JEI 1.21.1".into(),
            file_name: "jei-1.21.1.jar".into(),
            release_type: 1,
            file_length: 1024,
            download_count: 50,
            download_url: Some("https://example.com/jei.jar".into()),
            md5: Some("abc".into()),
            hashes: vec![Hash {
                value: "DEF".into(),
                algo: 1,
            }],
            game_versions: vec!["1.21.1".into()],
            dependencies: vec![],
            mod_loader: Some(loader::FABRIC),
            file_date: None,
            file_fingerprint: None,
        };
        let pv = to_project_version(&file);
        assert_eq!(pv.id, "12345");
        assert_eq!(pv.project_id, "curseforge-238222");
        assert_eq!(pv.loaders, vec!["fabric"]);
        assert_eq!(pv.files[0].url, "https://example.com/jei.jar");
        assert_eq!(pv.files[0].sha1(), Some("def"));
    }

    #[test]
    fn curseforge_fallback_cdn_and_game_version_loaders() {
        let file = CfFile {
            id: 5621345,
            mod_id: 100,
            display_name: "Mod".into(),
            file_name: "mod.jar".into(),
            release_type: 1,
            file_length: 500,
            download_count: 10,
            download_url: None,
            md5: None,
            hashes: vec![],
            game_versions: vec!["1.20.1".into(), "Fabric".into(), "Java 17".into()],
            dependencies: vec![],
            mod_loader: None,
            file_date: None,
            file_fingerprint: None,
        };
        let pv = to_project_version(&file);
        assert_eq!(pv.loaders, vec!["fabric"]);
        assert_eq!(pv.game_versions, vec!["1.20.1"]);
        assert_eq!(
            pv.files[0].url,
            "https://edge.forgecdn.net/files/5621/345/mod.jar"
        );
    }

    #[test]
    fn curseforge_fingerprint_calculation_matches_jei_known_hash() {
        let temp_jar = std::env::temp_dir().join("test_curseforge.jar");
        if temp_jar.exists() {
            let fp = curseforge_fingerprint_file(&temp_jar).unwrap();
            assert_eq!(fp, 1968017924);
        }
    }

    #[test]
    fn effective_url_resolution() {
        let client_default = CurseForgeClient::new(reqwest::Client::new(), "");
        assert_eq!(
            client_default.effective_url("/mods/search"),
            format!("{DEFAULT_SERVICE_URL}/mods/search")
        );

        let client_key = CurseForgeClient::new(reqwest::Client::new(), "my-key");
        assert_eq!(
            client_key.effective_url("/mods/search"),
            format!("{DIRECT_API_BASE}/mods/search")
        );

        let client_custom_proxy = CurseForgeClient::with_server(
            reqwest::Client::new(),
            "",
            "https://proxy.example.com/v1",
        );
        assert_eq!(
            client_custom_proxy.effective_url("/mods/search"),
            "https://proxy.example.com/v1/mods/search"
        );

        let client_custom_proxy_and_key = CurseForgeClient::with_server(
            reqwest::Client::new(),
            "my-key",
            "https://proxy.example.com/v1",
        );
        assert_eq!(
            client_custom_proxy_and_key.effective_url("/mods/search"),
            "https://proxy.example.com/v1/mods/search"
        );
    }
}
