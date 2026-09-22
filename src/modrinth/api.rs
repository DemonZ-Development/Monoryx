use crate::error::{MonoryxError, Result};
use crate::modrinth::models::{Project, ProjectVersion, SearchResponse};
use std::time::Duration;

pub const API_BASE: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Clone)]
pub struct ModrinthClient {
    http: reqwest::Client,
    ua: String,
}

impl ModrinthClient {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            ua: crate::utils::net::modrinth_user_agent(),
        }
    }

    fn cache(&self, dir: &std::path::Path) -> crate::storage::cache::DiskCache {
        crate::storage::cache::DiskCache::new(
            dir.join("metadata").join("modrinth"),
            Duration::from_secs(300),
        )
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let resp = self
                .http
                .get(url)
                .header(reqwest::header::USER_AGENT, &self.ua)
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
                tokio::time::sleep(Duration::from_secs(wait.min(10))).await;
                continue;
            }
            if status.is_server_error() && attempt < 4 {
                tokio::time::sleep(Duration::from_millis(400 * u64::from(attempt))).await;
                continue;
            }
            if !status.is_success() {
                return Err(MonoryxError::Modrinth(format!(
                    "Modrinth returned HTTP {status} for {url}"
                )));
            }
            return Ok(resp.json::<T>().await?);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn search(
        &self,
        query: &str,
        project_type: Option<&str>,
        game_version: Option<&str>,
        loader: Option<&str>,
        sort: &str,
        limit: u32,
        offset: u32,
    ) -> Result<SearchResponse> {
        let facets = build_facets(project_type, game_version, loader);
        let url = reqwest::Url::parse_with_params(
            &format!("{API_BASE}/search"),
            &[
                ("query", query),
                ("facets", &facets),
                ("index", sort),
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
            ],
        )
        .map_err(|e| MonoryxError::Modrinth(e.to_string()))?;
        self.get(url.as_str()).await
    }

    pub async fn project(&self, id_or_slug: &str) -> Result<Project> {
        self.get(&format!("{API_BASE}/project/{id_or_slug}")).await
    }

    pub async fn project_versions(
        &self,
        id_or_slug: &str,
        game_version: Option<&str>,
        loader: Option<&str>,
    ) -> Result<Vec<ProjectVersion>> {
        let mut url = format!("{API_BASE}/project/{id_or_slug}/version");
        let mut q = Vec::new();
        if let Some(g) = game_version {
            q.push(format!("game_versions=[\"{g}\"]"));
        }
        if let Some(l) = loader {
            q.push(format!("loaders=[\"{l}\"]"));
        }
        if !q.is_empty() {
            url.push('?');
            url.push_str(&q.join("&"));
        }
        self.get(&url).await
    }

    pub async fn version(&self, version_id: &str) -> Result<ProjectVersion> {
        self.get(&format!("{API_BASE}/version/{version_id}")).await
    }

    pub async fn versions_by_ids(&self, ids: &[String]) -> Result<Vec<ProjectVersion>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let body = serde_json::json!({ "ids": ids });
        let resp = self
            .http
            .post(format!("{API_BASE}/versions"))
            .header(reqwest::header::USER_AGENT, &self.ua)
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(MonoryxError::Modrinth(format!(
                "Modrinth returned HTTP {} for /versions",
                resp.status()
            )));
        }
        Ok(resp.json().await?)
    }

    pub async fn lookup_hashes(
        &self,
        hashes: &[String],
        algo: &str,
    ) -> Result<std::collections::HashMap<String, ProjectVersion>> {
        if hashes.is_empty() {
            return Ok(Default::default());
        }
        let body = serde_json::json!({ "hashes": hashes, "algorithm": algo });
        let resp = self
            .http
            .post(format!("{API_BASE}/version_files"))
            .header(reqwest::header::USER_AGENT, &self.ua)
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(MonoryxError::Modrinth(format!(
                "Modrinth returned HTTP {} for /version_files",
                resp.status()
            )));
        }
        Ok(resp.json().await?)
    }

    pub fn cached_project(&self, cache_dir: &std::path::Path, id: &str) -> Option<Project> {
        let bytes = self.cache(cache_dir).get(&format!("project-{id}"))?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn put_cached_project(&self, cache_dir: &std::path::Path, id: &str, p: &Project) {
        if let Ok(bytes) = serde_json::to_vec(p) {
            let _ = self.cache(cache_dir).put(&format!("project-{id}"), &bytes);
        }
    }
}

pub fn build_facets(
    project_type: Option<&str>,
    game_version: Option<&str>,
    loader: Option<&str>,
) -> String {
    let mut facets: Vec<Vec<String>> = Vec::new();
    if let Some(t) = project_type {
        facets.push(vec![format!("project_type:{t}")]);
    }
    if let Some(g) = game_version {
        if !g.is_empty() {
            facets.push(vec![format!("versions:{g}")]);
        }
    }
    if let Some(l) = loader {

        let is_pack_or_shader = matches!(project_type, Some("shader") | Some("resourcepack"));
        if !is_pack_or_shader && !l.is_empty() && l != "vanilla" && l != "minecraft" {
            facets.push(vec![format!("categories:{l}")]);
        }
    }
    serde_json::to_string(&facets).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facets_shape() {
        let f = build_facets(Some("mod"), Some("1.21"), Some("fabric"));
        let v: serde_json::Value = serde_json::from_str(&f).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 3);
    }

    #[test]
    fn vanilla_loader_omitted() {
        let f = build_facets(Some("mod"), Some("1.21"), Some("vanilla"));
        assert!(!f.contains("vanilla"));
    }

    #[test]
    fn shader_and_resourcepack_omit_loader_facet() {
        let shader = build_facets(Some("shader"), Some("1.21"), Some("fabric"));
        assert!(!shader.contains("categories:fabric"));
        assert!(shader.contains("project_type:shader"));
        assert!(shader.contains("versions:1.21"));

        let rp = build_facets(Some("resourcepack"), Some("1.21"), Some("fabric"));
        assert!(!rp.contains("categories:fabric"));
        assert!(rp.contains("project_type:resourcepack"));
    }

    #[test]
    fn modpack_supports_loader_facet() {
        let pack = build_facets(Some("modpack"), Some("1.21"), Some("fabric"));
        assert!(pack.contains("categories:fabric"));
        assert!(pack.contains("project_type:modpack"));
    }
}
