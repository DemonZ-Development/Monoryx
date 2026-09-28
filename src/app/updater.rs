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

fn newest_eligible_release(
    releases: Vec<GithubRelease>,
    current: &semver::Version,
) -> Option<(semver::Version, GithubRelease)> {
    releases
        .into_iter()
        .filter_map(|release| {
            let tag = release.tag_name.trim().trim_start_matches('v');
            let version = semver::Version::parse(tag).ok()?;
            (version > *current && (!current.pre.is_empty() || version.pre.is_empty()))
                .then_some((version, release))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
}

pub async fn check_launcher_update(http: &reqwest::Client) -> Result<LauncherUpdateInfo, String> {
    let current_str = env!("CARGO_PKG_VERSION");
    let current_ver = semver::Version::parse(current_str)
        .map_err(|e| format!("Invalid current version '{current_str}': {e}"))?;

    let url = "https://api.github.com/repos/DemonZ-Development/Monoryx/releases?per_page=30";
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

    if resp.status() == reqwest::StatusCode::FORBIDDEN
        || resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
    {
        return Err(
            "GitHub API rate limit reached. Please try checking again in a few minutes."
                .to_string(),
        );
    }

    if !resp.status().is_success() {
        return Err(format!(
            "Update server responded with status: {}",
            resp.status()
        ));
    }

    let releases: Vec<GithubRelease> = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse release metadata: {e}"))?;
    let release = newest_eligible_release(releases, &current_ver);

    let Some((latest_ver, release)) = release else {
        return Ok(LauncherUpdateInfo {
            current_version: current_str.to_string(),
            latest_version: current_str.to_string(),
            release_notes: "You are running the latest version of MONORYX.".to_string(),
            html_url: "https://github.com/DemonZ-Development/Monoryx/releases".to_string(),
            download_url: None,
            published_at: None,
            has_update: false,
        });
    };

    let download_url = release
        .assets
        .iter()
        .find(|asset| {
            let _name = asset.name.to_ascii_lowercase();
            #[cfg(target_os = "windows")]
            let matching = _name.starts_with("monoryx-setup-") && _name.ends_with(".exe");
            #[cfg(target_os = "linux")]
            let matching = _name.ends_with("-linux-x64.tar.gz");
            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            let matching = false;
            matching
        })
        .map(|asset| asset.browser_download_url.clone());

    Ok(LauncherUpdateInfo {
        current_version: current_str.to_string(),
        latest_version: latest_ver.to_string(),
        release_notes: release
            .body
            .unwrap_or_else(|| "No release notes provided for this update.".to_string()),
        html_url: release.html_url,
        download_url,
        published_at: release.published_at,
        has_update: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_release_json_with_update() {
        let sample = r#"{
            "tag_name": "v1.2.0",
            "html_url": "https://github.com/DemonZ-Development/Monoryx/releases/tag/v1.2.0",
            "body": "Major improvements and bug fixes!",
            "published_at": "2026-09-22T10:00:00Z",
            "assets": [
                {
                    "name": "MONORYX-Setup-1.2.0.exe",
                    "browser_download_url": "https://github.com/DemonZ-Development/Monoryx/releases/download/v1.2.0/MONORYX-Setup-1.2.0.exe"
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
            "https://github.com/DemonZ-Development/Monoryx/releases/download/v1.2.0/MONORYX-Setup-1.2.0.exe"
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

    #[test]
    fn update_selection_includes_beta_for_beta_users_only() {
        let releases: Vec<GithubRelease> = serde_json::from_str(r#"[
            {"tag_name":"v1.2.0-beta","html_url":"beta","body":null,"published_at":null,"assets":[]},
            {"tag_name":"v1.1.0","html_url":"stable","body":null,"published_at":null,"assets":[]},
            {"tag_name":"v1.0.0","html_url":"old","body":null,"published_at":null,"assets":[]}
        ]"#).unwrap();
        let beta = semver::Version::parse("1.1.0-beta").unwrap();
        assert_eq!(
            newest_eligible_release(releases, &beta)
                .unwrap()
                .0
                .to_string(),
            "1.2.0-beta"
        );

        let releases: Vec<GithubRelease> = serde_json::from_str(r#"[
            {"tag_name":"v1.2.0-beta","html_url":"beta","body":null,"published_at":null,"assets":[]},
            {"tag_name":"v1.1.0","html_url":"stable","body":null,"published_at":null,"assets":[]}
        ]"#).unwrap();
        let stable = semver::Version::parse("1.1.0").unwrap();
        assert!(newest_eligible_release(releases, &stable).is_none());
    }
}
