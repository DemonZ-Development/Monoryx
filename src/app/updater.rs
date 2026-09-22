use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherUpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: String,
    pub html_url: String,
    pub download_url: Option<String>,
    pub published_at: Option<String>,
    pub has_update: bool,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

pub async fn check_launcher_update(http: &reqwest::Client) -> Result<LauncherUpdateInfo, String> {
    let current_str = env!("CARGO_PKG_VERSION");
    let current_ver = semver::Version::parse(current_str)
        .map_err(|e| format!("Invalid current version '{current_str}': {e}"))?;

    let url = "https://api.github.com/repos/DemonZ-Development/Monoryx/releases/latest";
    let resp = http
        .get(url)
        .header("User-Agent", format!("MONORYX-Launcher/{current_str}"))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Failed to reach update server: {e}"))?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {

        return Ok(LauncherUpdateInfo {
            current_version: current_str.to_string(),
            latest_version: current_str.to_string(),
            release_notes: "You are running the latest version of MONORYX.".to_string(),
            html_url: "https://github.com/DemonZ-Development/Monoryx".to_string(),
            download_url: None,
            published_at: None,
            has_update: false,
        });
    }

    if resp.status() == reqwest::StatusCode::FORBIDDEN || resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err("GitHub API rate limit reached. Please try checking again in a few minutes.".to_string());
    }

    if !resp.status().is_success() {
        return Err(format!("Update server responded with status: {}", resp.status()));
    }

    let release: GithubRelease = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse release metadata: {e}"))?;

    let raw_tag = release.tag_name.trim();
    let clean_tag = raw_tag.strip_prefix('v').unwrap_or(raw_tag);
    let latest_ver = semver::Version::parse(clean_tag)
        .map_err(|e| format!("Invalid remote version tag '{raw_tag}': {e}"))?;

    let has_update = latest_ver > current_ver;

    let download_url = release
        .assets
        .iter()
        .find(|a| {
            let n = a.name.to_lowercase();
            n.ends_with(".exe") || n.ends_with(".msi") || n.ends_with(".zip")
        })
        .map(|a| a.browser_download_url.clone())
        .or_else(|| release.assets.first().map(|a| a.browser_download_url.clone()))
        .or_else(|| Some(release.html_url.clone()));

    Ok(LauncherUpdateInfo {
        current_version: current_str.to_string(),
        latest_version: clean_tag.to_string(),
        release_notes: release
            .body
            .unwrap_or_else(|| "No release notes provided for this update.".to_string()),
        html_url: release.html_url,
        download_url,
        published_at: release.published_at,
        has_update,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_release_json_with_update() {
        let sample = r#"{
            "tag_name": "v1.2.0",
            "html_url": "https://github.com/DemonZDevelopment/monoryx/releases/tag/v1.2.0",
            "body": "Major improvements and bug fixes!",
            "published_at": "2026-09-22T10:00:00Z",
            "assets": [
                {
                    "name": "monoryx-installer.exe",
                    "browser_download_url": "https://github.com/DemonZDevelopment/monoryx/releases/download/v1.2.0/monoryx-installer.exe"
                }
            ]
        }"#;

        let release: GithubRelease = serde_json::from_str(sample).unwrap();
        assert_eq!(release.tag_name, "v1.2.0");
        let clean = release.tag_name.strip_prefix('v').unwrap();
        let parsed = semver::Version::parse(clean).unwrap();
        assert!(parsed > semver::Version::parse("0.1.0").unwrap());
        assert_eq!(
            release.assets[0].browser_download_url,
            "https://github.com/DemonZDevelopment/monoryx/releases/download/v1.2.0/monoryx-installer.exe"
        );
    }

    #[test]
    fn semver_comparison_logic() {
        let current = semver::Version::parse("0.1.0").unwrap();
        let newer = semver::Version::parse("0.2.0").unwrap();
        let patch = semver::Version::parse("0.1.1").unwrap();
        let older = semver::Version::parse("0.0.9").unwrap();
        let same = semver::Version::parse("0.1.0").unwrap();

        assert!(newer > current);
        assert!(patch > current);
        assert!(!(older > current));
        assert!(!(same > current));
    }

    #[test]
    fn parse_release_without_v_prefix_and_older_version() {
        let sample = r#"{
            "tag_name": "0.0.5",
            "html_url": "https://github.com/DemonZ-Development/Monoryx/releases/tag/0.0.5",
            "body": null,
            "published_at": null,
            "assets": []
        }"#;

        let release: GithubRelease = serde_json::from_str(sample).unwrap();
        assert_eq!(release.tag_name, "0.0.5");
        let parsed = semver::Version::parse(&release.tag_name).unwrap();
        let current = semver::Version::parse("0.1.0").unwrap();
        assert!(!(parsed > current));
    }
}
