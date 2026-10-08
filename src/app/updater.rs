use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsUpdateKind {
    Binary,
    Installer,
}

pub fn windows_update_kind(filename: &str) -> Option<WindowsUpdateKind> {
    let name = filename.to_ascii_lowercase();
    if name == "monoryx-update.exe" || name == "monoryx.exe" {
        Some(WindowsUpdateKind::Binary)
    } else if name
        .strip_prefix("monoryx-setup-")
        .and_then(|suffix| suffix.strip_suffix(".exe"))
        .is_some_and(|version| semver::Version::parse(version).is_ok())
    {
        Some(WindowsUpdateKind::Installer)
    } else {
        None
    }
}

pub fn validated_update_filename(url: &str, version: &str) -> Result<String, String> {
    let parsed = url::Url::parse(url).map_err(|_| "Invalid update URL.".to_string())?;
    let segments: Vec<_> = parsed
        .path_segments()
        .ok_or("Invalid update URL.")?
        .collect();
    let expected_version =
        semver::Version::parse(version).map_err(|_| "Invalid update version.".to_string())?;
    if parsed.scheme() != "https"
        || parsed.port().is_some()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err("Invalid update asset URL.".into());
    }
    let filename = match parsed.host_str() {
        Some("demonz.org") if segments.len() == 3 && segments[..2] == ["api", "downloads"] => {
            segments[2]
        }
        Some("github.com")
            if segments.len() == 6
                && segments[0].eq_ignore_ascii_case("DemonZ-Development")
                && segments[1].eq_ignore_ascii_case("Monoryx")
                && segments[2..4] == ["releases", "download"]
                && semver::Version::parse(segments[4].strip_prefix('v').unwrap_or(segments[4]))
                    .ok()
                    .as_ref()
                    == Some(&expected_version) =>
        {
            segments[5]
        }
        _ => return Err("Update asset URL does not match an official MONORYX release.".into()),
    };
    crate::utils::fs::safe_file_name(filename).map_err(|e| e.user_message())?;
    let kind = windows_update_kind(filename).ok_or("Unsupported update asset.")?;
    if kind == WindowsUpdateKind::Installer
        && !filename.eq_ignore_ascii_case(&format!("MONORYX-Setup-{expected_version}.exe"))
    {
        return Err("Update installer version does not match the release.".into());
    }
    Ok(filename.to_string())
}

pub fn parse_sha256(text: &str, filename: &str, allow_bare: bool) -> Option<String> {
    let mut matching = None;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let mut fields = line.split_whitespace();
        let hash = fields.next()?;
        let name = fields.next();
        if fields.next().is_some() {
            continue;
        }
        if name
            .map(|name| name.trim_start_matches('*') == filename)
            .unwrap_or(allow_bare)
            && hash.len() == 64
            && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            if matching.is_some() {
                return None;
            }
            matching = Some(hash.to_ascii_lowercase());
        }
    }
    matching
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherUpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: String,
    pub html_url: String,
    pub download_url: Option<String>,
    pub download_asset: Option<LauncherUpdateAsset>,
    pub published_at: Option<String>,
    pub has_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherUpdateAsset {
    pub direct_url: String,
    pub file_name: String,
    pub file_size: Option<u64>,
    pub sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateResponse {
    project: UpdateProject,
    latest_version: String,
    platform: Option<String>,
    release: Option<UpdateRelease>,
    download: Option<UpdateDownload>,
    changelog: Option<UpdateChangelog>,
}

#[derive(Debug, Deserialize)]
struct UpdateProject {
    slug: String,
}

#[derive(Debug, Deserialize)]
struct UpdateRelease {
    version: String,
    released_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateDownload {
    direct_url: Option<String>,
    file_name: Option<String>,
    file_size: Option<u64>,
    checksum: Option<String>,
    hash_algorithm: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateChangelog {
    latest: Option<UpdateNotes>,
}

#[derive(Debug, Deserialize)]
struct UpdateNotes {
    version: String,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FilesResponse {
    slug: String,
    files: Vec<ReleaseFile>,
}

#[derive(Debug, Deserialize)]
struct ReleaseFile {
    platform: String,
    version: String,
    download_url: String,
    file_name: Option<String>,
    checksum: Option<String>,
    hash_algorithm: Option<String>,
}

fn release_version(value: &str) -> Result<semver::Version, String> {
    semver::Version::parse(value.trim().strip_prefix('v').unwrap_or(value.trim()))
        .map_err(|_| format!("Invalid update version '{value}'."))
}

fn api_sha256(value: Option<&str>, algorithm: Option<&str>) -> Result<Option<String>, String> {
    let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    if algorithm.is_some_and(|algorithm| {
        !algorithm.eq_ignore_ascii_case("sha256") && !algorithm.eq_ignore_ascii_case("sha-256")
    }) {
        return Err("The update server published an unsupported checksum algorithm.".into());
    }
    parse_sha256(value.trim(), "", true)
        .filter(|_| !value.trim().contains(char::is_whitespace))
        .map(Some)
        .ok_or_else(|| "The update server published an invalid SHA-256 checksum.".into())
}

pub(super) fn download_url(platform: &str, version: &semver::Version) -> String {
    let mut url = url::Url::parse(crate::utils::links::DOWNLOAD_API_URL).unwrap();
    url.query_pairs_mut()
        .append_pair("platform", platform)
        .append_pair("version", &version.to_string());
    url.into()
}

fn update_info(
    response: UpdateResponse,
    current: &str,
    platform: &str,
) -> Result<LauncherUpdateInfo, String> {
    if response.project.slug != "monoryx" {
        return Err("The update server returned a different project's metadata.".into());
    }
    if response
        .platform
        .as_deref()
        .is_some_and(|returned| returned != platform)
    {
        return Err("The update server returned metadata for a different platform.".into());
    }
    let current_version = release_version(current)?;
    let latest_version = release_version(&response.latest_version)?;
    let has_update = latest_version.cmp_precedence(&current_version).is_gt()
        && (!current_version.pre.is_empty() || latest_version.pre.is_empty());
    let mut info = LauncherUpdateInfo {
        current_version: current_version.to_string(),
        latest_version: current_version.to_string(),
        release_notes: "You are running the latest version of MONORYX.".into(),
        html_url: crate::utils::links::PROJECT_URL.into(),
        download_url: None,
        download_asset: None,
        published_at: None,
        has_update,
    };
    if !has_update {
        return Ok(info);
    }
    if let Some(release) = response.release {
        if release_version(&release.version)? != latest_version {
            return Err("Update release metadata does not match the advertised version.".into());
        }
        info.published_at = release.released_at;
    }
    info.latest_version = latest_version.to_string();
    info.release_notes = response
        .changelog
        .and_then(|changelog| changelog.latest)
        .filter(|notes| release_version(&notes.version).ok().as_ref() == Some(&latest_version))
        .and_then(|notes| notes.notes)
        .unwrap_or_else(|| "No release notes provided for this update.".into());
    if platform != "windows" {
        info.download_url = Some(crate::utils::links::PROJECT_URL.into());
    } else if let Some(download) = response.download {
        if let (Some(direct_url), Some(file_name)) = (download.direct_url, download.file_name) {
            if validated_update_filename(&direct_url, &info.latest_version)? != file_name {
                return Err("Update filename does not match its release asset URL.".into());
            }
            let sha256 = api_sha256(
                download.checksum.as_deref(),
                download.hash_algorithm.as_deref(),
            )?;
            info.download_url = Some(download_url(platform, &latest_version));
            info.download_asset = Some(LauncherUpdateAsset {
                direct_url,
                file_name,
                file_size: download.file_size,
                sha256,
            });
        }
    }
    Ok(info)
}

async fn api_json<T: serde::de::DeserializeOwned>(
    http: &reqwest::Client,
    url: &str,
    query: &[(&str, &str)],
) -> Result<T, String> {
    let response = http
        .get(url)
        .query(query)
        .header("Accept", "application/json")
        .header("User-Agent", crate::utils::net::MONORYX_USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Failed to reach the official update server: {e}"))?;
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err("The official update server is busy. Try again in a few minutes.".into());
    }
    if !response.status().is_success() {
        return Err(format!(
            "Official update server returned HTTP {}.",
            response.status()
        ));
    }
    response
        .json()
        .await
        .map_err(|e| format!("Invalid update metadata: {e}"))
}

fn file_checksum(
    response: FilesResponse,
    asset: &LauncherUpdateAsset,
    version: &str,
) -> Result<Option<String>, String> {
    if response.slug != "monoryx" {
        return Err("The update server returned a different project's files.".into());
    }
    let version = release_version(version)?;
    let matching = response.files.into_iter().filter(|file| {
        file.platform == "windows"
            && release_version(&file.version).ok().as_ref() == Some(&version)
            && file.download_url == asset.direct_url
            && file.file_name.as_deref() == Some(asset.file_name.as_str())
    });
    let mut checksum = None;
    for file in matching {
        if let Some(hash) = api_sha256(file.checksum.as_deref(), file.hash_algorithm.as_deref())? {
            if checksum.as_ref().is_some_and(|previous| previous != &hash) {
                return Err("The update server returned conflicting release checksums.".into());
            }
            checksum = Some(hash);
        }
    }
    Ok(checksum)
}

fn api_platform(os: &str) -> &str {
    match os {
        "macos" => "mac",
        os => os,
    }
}

pub async fn check_launcher_update(http: &reqwest::Client) -> Result<LauncherUpdateInfo, String> {
    let platform = api_platform(std::env::consts::OS);
    check_launcher_update_from(
        http,
        crate::utils::links::UPDATE_API_URL,
        crate::utils::links::FILES_API_URL,
        env!("CARGO_PKG_VERSION"),
        platform,
    )
    .await
}

async fn check_launcher_update_from(
    http: &reqwest::Client,
    updates_url: &str,
    files_url: &str,
    current: &str,
    platform: &str,
) -> Result<LauncherUpdateInfo, String> {
    let response = api_json(
        http,
        updates_url,
        &[("current_version", current), ("platform", platform)],
    )
    .await?;
    let mut info = update_info(response, current, platform)?;
    if let Some(asset) = info
        .download_asset
        .as_mut()
        .filter(|asset| asset.sha256.is_none())
    {
        match api_json(http, files_url, &[]).await {
            Ok(files) => asset.sha256 = file_checksum(files, asset, &info.latest_version)?,
            Err(error) => tracing::warn!("Could not load release file checksums: {error}"),
        }
    }
    Ok(info)
}

#[cfg(any(target_os = "windows", test))]
pub(super) async fn fetch_expected_sha256(
    http: &reqwest::Client,
    url: &str,
    filename: &str,
) -> Result<String, String> {
    let direct_url = format!("{url}.sha256");
    if let Ok(resp) = http
        .get(&direct_url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(text) = resp.text().await {
                if let Some(hash) = parse_sha256(&text, filename, true) {
                    return Ok(hash);
                }
            }
        }
    }
    if let Some((base, _)) = url.rsplit_once('/') {
        let sums_url = format!("{base}/SHA256SUMS.txt");
        if let Ok(resp) = http
            .get(&sums_url)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if let Some(hash) = parse_sha256(&text, filename, false) {
                        return Ok(hash);
                    }
                }
            }
        }
    }
    Err("No valid SHA-256 checksum was published for this update. Download was stopped.".into())
}

pub fn apply_update_and_restart(update_path: &std::path::Path) -> std::io::Result<()> {
    let current_exe = std::env::current_exe()?;
    let current_dir = current_exe
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let pid = std::process::id();

    let file_name = update_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let companion_name = if cfg!(target_os = "windows") {
        "monoryx-updater.exe"
    } else {
        "monoryx-updater"
    };
    let companion = current_dir.join(companion_name);

    let kind = windows_update_kind(file_name).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Unsupported update asset")
    })?;
    if kind == WindowsUpdateKind::Installer {
        let mut cmd = std::process::Command::new(update_path);
        cmd.args([
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/SP-",
            "/MERGETASKS=\"\"",
        ]);
        cmd.spawn()?;
        std::process::exit(0);
    }

    let updater_exe = if companion.exists() {
        companion
    } else {
        let temp_updater = std::env::temp_dir().join(format!("monoryx-updater-{pid}.exe"));
        std::fs::copy(&current_exe, &temp_updater)?;
        temp_updater
    };

    let mut cmd = std::process::Command::new(&updater_exe);
    cmd.current_dir(current_dir)
        .arg("--update-source")
        .arg(update_path)
        .arg("--target-dest")
        .arg(&current_exe)
        .arg("--wait-pid")
        .arg(pid.to_string())
        .arg("--relaunch");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.spawn()?;
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_assets_keep_their_type_and_validate_release_url() {
        let base = "https://github.com/DemonZ-Development/Monoryx/releases/download/v1.6.0";
        assert_eq!(
            validated_update_filename(&format!("{base}/monoryx-update.exe"), "1.6.0").unwrap(),
            "monoryx-update.exe"
        );
        assert_eq!(
            windows_update_kind("monoryx-update.exe"),
            Some(WindowsUpdateKind::Binary)
        );
        assert_eq!(
            windows_update_kind("MONORYX-Setup-1.6.0.exe"),
            Some(WindowsUpdateKind::Installer)
        );
        for url in [
            format!("{base}/MONORYX-Setup-1.5.0.exe"),
            format!("{base}/monoryx-updater.exe"),
            format!("{base}/monoryx-update.exe?other=1"),
            base.replace("github.com", "github.com.evil.test") + "/monoryx-update.exe",
        ] {
            assert!(validated_update_filename(&url, "1.6.0").is_err());
        }
        assert!(validated_update_filename(&format!("{base}/monoryx-update.exe"), "1.5.0").is_err());
    }

    #[test]
    fn checksum_manifest_requires_exact_unambiguous_filename() {
        let hash = "a".repeat(64);
        assert_eq!(
            parse_sha256(
                &format!("{hash}  monoryx-update.exe\n"),
                "monoryx-update.exe",
                false
            ),
            Some(hash.clone())
        );
        assert_eq!(
            parse_sha256(&hash, "monoryx-update.exe", true),
            Some(hash.clone())
        );
        assert!(parse_sha256(&hash, "monoryx-update.exe", false).is_none());
        assert!(parse_sha256(
            &format!("{hash}  other-monoryx-update.exe\n"),
            "monoryx-update.exe",
            false
        )
        .is_none());
        assert!(parse_sha256(
            &format!("{hash}  monoryx-update.exe\n{hash}  monoryx-update.exe\n"),
            "monoryx-update.exe",
            false
        )
        .is_none());
        assert!(parse_sha256("broken monoryx-update.exe", "monoryx-update.exe", false).is_none());
    }

    #[test]
    fn official_hosted_assets_validate_host_path_type_and_installer_version() {
        for filename in ["MONORYX-Setup-1.6.0.exe", "monoryx-update.exe"] {
            assert_eq!(
                validated_update_filename(
                    &format!("https://demonz.org/api/downloads/{filename}"),
                    "1.6.0"
                )
                .unwrap(),
                filename
            );
        }
        for url in [
            "https://demonz.org/api/downloads/MONORYX-Setup-1.5.0.exe",
            "https://demonz.org/api/downloads/monoryx-updater.exe",
            "https://demonz.org/api/downloads/MONORYX-Setup-1.6.0.exe?version=other",
            "http://demonz.org/api/downloads/MONORYX-Setup-1.6.0.exe",
            "https://demonz.org.evil.test/api/downloads/MONORYX-Setup-1.6.0.exe",
            "https://demonz.org/other/MONORYX-Setup-1.6.0.exe",
            "https://demonz.org/api/downloads/nested/MONORYX-Setup-1.6.0.exe",
            "https://user@demonz.org/api/downloads/MONORYX-Setup-1.6.0.exe",
        ] {
            assert!(validated_update_filename(url, "1.6.0").is_err(), "{url}");
        }
        let mut value = response("1.6.0");
        value["download"]["direct_url"] =
            serde_json::json!("https://demonz.org/api/downloads/MONORYX-Setup-1.6.0.exe");
        assert_eq!(
            info(value, "1.5.0")
                .unwrap()
                .download_asset
                .unwrap()
                .direct_url,
            "https://demonz.org/api/downloads/MONORYX-Setup-1.6.0.exe"
        );
    }

    fn response(version: &str) -> serde_json::Value {
        serde_json::json!({
            "project": {"slug": "monoryx"},
            "latest_version": version,
            "has_update": true,
            "release": {"version": version, "released_at": "2026-10-05T00:00:00Z"},
            "download": {
                "url": "https://demonz.org/api/projects/download/monoryx?platform=windows",
                "direct_url": format!("https://github.com/DemonZ-Development/Monoryx/releases/download/v{version}/MONORYX-Setup-{version}.exe"),
                "file_name": format!("MONORYX-Setup-{version}.exe"),
                "file_size": 7684896,
                "checksum": null,
                "hash_algorithm": null
            },
            "changelog": {"latest": {"version": version, "notes": "Release notes from the official API."}}
        })
    }

    fn info(value: serde_json::Value, current: &str) -> Result<LauncherUpdateInfo, String> {
        update_info(serde_json::from_value(value).unwrap(), current, "windows")
    }

    #[test]
    fn api_cannot_offer_an_older_or_equal_version_as_an_update() {
        for version in ["1.4.0", "1.5.0", "1.5.0+build.1"] {
            let result = info(response(version), "1.5.0").unwrap();
            assert!(!result.has_update);
            assert_eq!(result.latest_version, "1.5.0");
            assert!(result.download_url.is_none());
            assert!(result.download_asset.is_none());
            assert_eq!(result.html_url, crate::utils::links::PROJECT_URL);
        }
    }

    #[test]
    fn api_update_keeps_release_metadata_and_pins_official_download_version() {
        let mut value = response("1.6.0");
        value["has_update"] = serde_json::json!(false);
        let result = info(value, "1.5.0").unwrap();
        assert!(result.has_update);
        assert_eq!(result.latest_version, "1.6.0");
        assert_eq!(result.release_notes, "Release notes from the official API.");
        assert_eq!(result.published_at.as_deref(), Some("2026-10-05T00:00:00Z"));
        assert_eq!(
            result.download_url.as_deref(),
            Some("https://demonz.org/api/projects/download/monoryx?platform=windows&version=1.6.0")
        );
        assert_eq!(
            result.download_asset.unwrap().file_name,
            "MONORYX-Setup-1.6.0.exe"
        );
    }

    #[test]
    fn stable_users_only_receive_stable_updates() {
        assert!(!info(response("1.6.0-beta"), "1.5.0").unwrap().has_update);
        assert!(
            info(response("1.6.0-beta"), "1.5.0-beta")
                .unwrap()
                .has_update
        );
        assert!(info(response("1.5.0"), "1.5.0-beta").unwrap().has_update);
        let mut value = response("1.6.0");
        value["latest_version"] = serde_json::json!("v1.6.0");
        assert_eq!(info(value, "1.5.0").unwrap().latest_version, "1.6.0");
    }

    #[test]
    fn malformed_or_mismatched_release_metadata_is_rejected() {
        for (pointer, replacement) in [
            ("/project/slug", "different-project"),
            ("/latest_version", "latest"),
            ("/release/version", "1.7.0"),
            ("/download/file_name", "monoryx-update.exe"),
            (
                "/download/direct_url",
                "https://evil.test/MONORYX-Setup-1.6.0.exe",
            ),
        ] {
            let mut value = response("1.6.0");
            *value.pointer_mut(pointer).unwrap() = serde_json::json!(replacement);
            assert!(info(value, "1.5.0").is_err(), "{pointer}");
        }
    }

    #[test]
    fn api_checksums_are_validated_before_download() {
        let mut value = response("1.6.0");
        value["download"]["checksum"] = serde_json::json!("A".repeat(64));
        value["download"]["hash_algorithm"] = serde_json::json!("SHA-256");
        assert_eq!(
            info(value.clone(), "1.5.0")
                .unwrap()
                .download_asset
                .unwrap()
                .sha256,
            Some("a".repeat(64))
        );
        value["download"]["hash_algorithm"] = serde_json::json!("md5");
        assert!(info(value.clone(), "1.5.0").is_err());
        value["download"]["hash_algorithm"] = serde_json::Value::Null;
        for invalid in ["invalid".into(), format!("{}  file.exe", "a".repeat(64))] {
            value["download"]["checksum"] = serde_json::json!(invalid);
            assert!(info(value.clone(), "1.5.0").is_err());
        }
    }

    #[test]
    fn mismatched_platform_metadata_is_rejected() {
        let mut value = response("1.6.0");
        value["platform"] = serde_json::json!("linux");
        assert!(info(value, "1.5.0").is_err());
    }

    #[test]
    fn missing_assets_and_stale_notes_do_not_create_invalid_downloads() {
        let mut value = response("1.6.0");
        value["download"] = serde_json::Value::Null;
        value["changelog"]["latest"]["version"] = serde_json::json!("1.4.0");
        let result = info(value, "1.5.0").unwrap();
        assert!(result.has_update);
        assert!(result.download_url.is_none());
        assert!(result.download_asset.is_none());
        assert_eq!(
            result.release_notes,
            "No release notes provided for this update."
        );
    }

    #[test]
    fn non_windows_downloads_open_the_official_platform_picker() {
        for platform in ["linux", "mac"] {
            let result = update_info(
                serde_json::from_value(response("1.6.0")).unwrap(),
                "1.5.0",
                platform,
            )
            .unwrap();
            assert_eq!(
                result.download_url.as_deref(),
                Some(crate::utils::links::PROJECT_URL)
            );
            assert!(result.download_asset.is_none());
        }
    }

    fn files(asset: &LauncherUpdateAsset) -> serde_json::Value {
        serde_json::json!({
            "slug": "monoryx",
            "files": [{
                "platform": "windows", "version": "1.6.0",
                "download_url": asset.direct_url, "file_name": asset.file_name,
                "checksum": "b".repeat(64)
            }]
        })
    }

    #[test]
    fn file_checksum_is_bound_to_the_exact_project_version_and_asset() {
        let asset = info(response("1.6.0"), "1.5.0")
            .unwrap()
            .download_asset
            .unwrap();
        let value = files(&asset);
        assert_eq!(
            file_checksum(
                serde_json::from_value(value.clone()).unwrap(),
                &asset,
                "1.6.0"
            )
            .unwrap(),
            Some("b".repeat(64))
        );
        for (field, replacement) in [
            ("platform", "mac"),
            ("version", "1.5.0"),
            ("download_url", "https://evil.test/asset.exe"),
            ("file_name", "other.exe"),
        ] {
            let mut value = value.clone();
            value["files"][0][field] = serde_json::json!(replacement);
            assert_eq!(
                file_checksum(serde_json::from_value(value).unwrap(), &asset, "1.6.0").unwrap(),
                None
            );
        }
        let mut duplicate = value.clone();
        duplicate["files"]
            .as_array_mut()
            .unwrap()
            .push(value["files"][0].clone());
        assert_eq!(
            file_checksum(serde_json::from_value(duplicate).unwrap(), &asset, "1.6.0").unwrap(),
            Some("b".repeat(64))
        );
        let mut conflicting = value.clone();
        let mut other = value["files"][0].clone();
        other["checksum"] = serde_json::json!("c".repeat(64));
        conflicting["files"].as_array_mut().unwrap().push(other);
        assert!(file_checksum(
            serde_json::from_value(conflicting).unwrap(),
            &asset,
            "1.6.0"
        )
        .is_err());
        let mut missing_hash = value.clone();
        missing_hash["files"][0]["checksum"] = serde_json::Value::Null;
        let repeated = missing_hash["files"][0].clone();
        missing_hash["files"].as_array_mut().unwrap().push(repeated);
        assert_eq!(
            file_checksum(
                serde_json::from_value(missing_hash).unwrap(),
                &asset,
                "1.6.0"
            )
            .unwrap(),
            None
        );
        let mut wrong_project = value;
        wrong_project["slug"] = serde_json::json!("other");
        assert!(file_checksum(
            serde_json::from_value(wrong_project).unwrap(),
            &asset,
            "1.6.0"
        )
        .is_err());
    }

    async fn serve(
        responses: Vec<(&'static str, &'static str, String)>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            for (path, status, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buffer = [0; 4096];
                let mut length = 0;
                loop {
                    assert!(length < buffer.len());
                    let count = stream.read(&mut buffer[length..]).await.unwrap();
                    assert!(count > 0);
                    length += count;
                    if buffer[..length].windows(4).any(|part| part == b"\r\n\r\n") {
                        break;
                    }
                }
                let request = std::str::from_utf8(&buffer[..length]).unwrap();
                assert!(
                    request.starts_with(&format!("GET {path} HTTP/1.1\r\n")),
                    "{request}"
                );
                let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        });
        (base, task)
    }

    #[tokio::test]
    async fn official_api_request_sends_version_and_platform_and_loads_file_checksum() {
        let value = response("1.6.0");
        let asset = info(value.clone(), "1.5.0")
            .unwrap()
            .download_asset
            .unwrap();
        let (base, task) = serve(vec![
            (
                "/updates?current_version=1.5.0&platform=windows",
                "200 OK",
                value.to_string(),
            ),
            ("/files", "200 OK", files(&asset).to_string()),
        ])
        .await;
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let result = check_launcher_update_from(
            &client,
            &format!("{base}/updates"),
            &format!("{base}/files"),
            "1.5.0",
            "windows",
        )
        .await
        .unwrap();
        assert!(result.has_update);
        assert_eq!(result.download_asset.unwrap().sha256, Some("b".repeat(64)));
        task.await.unwrap();
    }

    #[tokio::test]
    async fn platform_requests_use_the_api_keys_for_every_supported_os() {
        for (os, path, returned) in [
            (
                "windows",
                "/updates?current_version=1.5.0&platform=windows",
                "windows",
            ),
            (
                "macos",
                "/updates?current_version=1.5.0&platform=mac",
                "mac",
            ),
            (
                "linux",
                "/updates?current_version=1.5.0&platform=linux",
                "linux",
            ),
        ] {
            let mut value = response("1.6.0");
            value["platform"] = serde_json::json!(returned);
            value["download"]["checksum"] = serde_json::json!("a".repeat(64));
            let (base, task) = serve(vec![(path, "200 OK", value.to_string())]).await;
            let client = reqwest::Client::builder().no_proxy().build().unwrap();
            let result = check_launcher_update_from(
                &client,
                &format!("{base}/updates"),
                "http://127.0.0.1:1/files",
                "1.5.0",
                api_platform(os),
            )
            .await
            .unwrap();
            assert!(result.has_update);
            if os == "windows" {
                assert!(result.download_asset.is_some());
            } else {
                assert_eq!(
                    result.download_url.as_deref(),
                    Some(crate::utils::links::PROJECT_URL)
                );
            }
            task.await.unwrap();
        }
    }

    #[tokio::test]
    async fn update_http_errors_and_html_are_not_reported_as_up_to_date() {
        for (status, body, expected) in [
            ("404 Not Found", "{}", "404"),
            ("429 Too Many Requests", "{}", "busy"),
            ("200 OK", "<html>site</html>", "Invalid update metadata"),
        ] {
            let (base, task) = serve(vec![(
                "/updates?current_version=1.5.0&platform=windows",
                status,
                body.into(),
            )])
            .await;
            let client = reqwest::Client::builder().no_proxy().build().unwrap();
            let error = check_launcher_update_from(
                &client,
                &format!("{base}/updates"),
                &format!("{base}/files"),
                "1.5.0",
                "windows",
            )
            .await
            .unwrap_err();
            assert!(error.contains(expected), "{error}");
            task.await.unwrap();
        }
    }

    #[tokio::test]
    async fn current_release_and_embedded_checksum_skip_the_files_request() {
        let mut checked = response("1.6.0");
        checked["download"]["checksum"] = serde_json::json!("a".repeat(64));
        for value in [response("1.4.0"), checked] {
            let (base, task) = serve(vec![(
                "/updates?current_version=1.5.0&platform=windows",
                "200 OK",
                value.to_string(),
            )])
            .await;
            let client = reqwest::Client::builder().no_proxy().build().unwrap();
            check_launcher_update_from(
                &client,
                &format!("{base}/updates"),
                "http://127.0.0.1:1/files",
                "1.5.0",
                "windows",
            )
            .await
            .unwrap();
            task.await.unwrap();
        }
    }

    #[tokio::test]
    async fn missing_file_metadata_keeps_checksum_sidecar_fallback_available() {
        let (base, task) = serve(vec![
            (
                "/updates?current_version=1.5.0&platform=windows",
                "200 OK",
                response("1.6.0").to_string(),
            ),
            ("/files", "503 Service Unavailable", "{}".into()),
        ])
        .await;
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let result = check_launcher_update_from(
            &client,
            &format!("{base}/updates"),
            &format!("{base}/files"),
            "1.5.0",
            "windows",
        )
        .await
        .unwrap();
        assert!(result.has_update);
        assert!(result.download_asset.unwrap().sha256.is_none());
        task.await.unwrap();
    }

    #[tokio::test]
    async fn checksum_sidecar_and_manifest_fallback_require_the_selected_asset() {
        let hash = "c".repeat(64);
        for (sidecar, manifest, expected) in [
            (hash.clone(), None, true),
            (
                "missing".into(),
                Some(format!("{hash}  monoryx-update.exe\n")),
                true,
            ),
            (
                "missing".into(),
                Some(format!("{hash}  other.exe\n")),
                false,
            ),
        ] {
            let mut responses = vec![("/monoryx-update.exe.sha256", "200 OK", sidecar)];
            if let Some(manifest) = manifest {
                responses.push(("/SHA256SUMS.txt", "200 OK", manifest));
            }
            let (base, task) = serve(responses).await;
            let client = reqwest::Client::builder().no_proxy().build().unwrap();
            let result = fetch_expected_sha256(
                &client,
                &format!("{base}/monoryx-update.exe"),
                "monoryx-update.exe",
            )
            .await;
            if expected {
                assert_eq!(result.unwrap(), hash);
            } else {
                assert!(result.unwrap_err().contains("No valid SHA-256"));
            }
            task.await.unwrap();
        }
    }
}
