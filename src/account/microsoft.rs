use crate::error::{MonoryxError, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MicrosoftProfile {
    pub username: String,
    pub uuid: uuid::Uuid,
    #[serde(default, skip_serializing)]
    pub access_token: String,
    #[serde(default, skip_serializing)]
    pub refresh_token: String,
    pub expires_at: i64,
}

impl std::fmt::Debug for MicrosoftProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MicrosoftProfile")
            .field("username", &self.username)
            .field("uuid", &self.uuid)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct DeviceErrorResponse {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxAuthResponse {
    token: String,
    display_claims: DisplayClaims,
}

#[derive(Debug, Deserialize)]
struct DisplayClaims {
    xui: Vec<XuiClaim>,
}

#[derive(Debug, Deserialize)]
struct XuiClaim {
    uhs: String,
}

#[derive(Debug, Deserialize)]
pub struct MinecraftAuthResponse {
    pub access_token: String,
    #[serde(default)]
    pub expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct MinecraftProfileResponse {
    pub id: String,
    pub name: String,
}

pub async fn request_device_code(
    http: &reqwest::Client,
    client_id: &str,
) -> Result<DeviceCodeResponse> {
    let id = if client_id.trim().is_empty() {
        DEFAULT_CLIENT_ID
    } else {
        client_id.trim()
    };
    let params = [
        ("client_id", id),
        ("scope", "XboxLive.signin offline_access"),
    ];
    let resp = http
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .form(&params)
        .send()
        .await
        .map_err(|e| MonoryxError::Auth(format!("Failed to request Microsoft device code: {e}")))?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(MonoryxError::Auth(format!(
            "Microsoft device code error: {err_text}"
        )));
    }

    resp.json::<DeviceCodeResponse>()
        .await
        .map_err(|e| MonoryxError::Auth(format!("Invalid device code response: {e}")))
}

pub async fn poll_device_token(
    http: &reqwest::Client,
    client_id: &str,
    device_code: &str,
) -> Result<Option<DeviceTokenResponse>> {
    let id = if client_id.trim().is_empty() {
        DEFAULT_CLIENT_ID
    } else {
        client_id.trim()
    };
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("client_id", id),
        ("device_code", device_code),
    ];
    let resp = http
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| MonoryxError::Auth(format!("Failed to poll Microsoft token: {e}")))?;

    if resp.status().is_success() {
        let tok = resp
            .json::<DeviceTokenResponse>()
            .await
            .map_err(|e| MonoryxError::Auth(format!("Failed to parse token response: {e}")))?;
        return Ok(Some(tok));
    }

    if let Ok(err_body) = resp.json::<DeviceErrorResponse>().await {
        if err_body.error == "authorization_pending" {
            return Ok(None);
        }
        if err_body.error == "authorization_declined" {
            return Err(MonoryxError::Auth(
                "Login was cancelled on the browser.".into(),
            ));
        }
        if err_body.error == "expired_token" {
            return Err(MonoryxError::Auth(
                "Login code expired. Please try signing in again.".into(),
            ));
        }
        return Err(MonoryxError::Auth(format!(
            "Microsoft login error: {}",
            err_body.error_description.unwrap_or(err_body.error)
        )));
    }

    Err(MonoryxError::Auth(
        "Failed to retrieve Microsoft authorization token.".into(),
    ))
}

pub async fn login_with_xbox(
    http: &reqwest::Client,
    user_hash: &str,
    xsts_token: &str,
) -> Result<MinecraftAuthResponse> {
    let mc_body = serde_json::json!({
        "identityToken": format!("XBL3.0 x={user_hash};{xsts_token}")
    });

    let mc_resp = http
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&mc_body)
        .send()
        .await
        .map_err(|e| MonoryxError::Auth(format!("Minecraft services login failed: {e}")))?;

    if !mc_resp.status().is_success() {
        let status = mc_resp.status();
        let err_text = mc_resp.text().await.unwrap_or_default();
        if is_not_found(status, &err_text) {
            return Err(MonoryxError::Auth(
                "This Microsoft account does not own Minecraft Java Edition (or no profile name has been set yet at minecraft.net).".into(),
            ));
        }
        if status == reqwest::StatusCode::FORBIDDEN || err_text.contains("Invalid app registration")
        {
            return Err(MonoryxError::Auth(
                "Mojang has not approved this custom Azure client ID yet (submit for approval at aka.ms/mce-reviewappid). Clear your custom client ID in Settings to use the default approved launcher ID.".into(),
            ));
        }
        let msg = format_mojang_error_detail(status, &err_text);
        return Err(MonoryxError::Auth(format!(
            "Failed to obtain Minecraft access token from Mojang services: {msg}"
        )));
    }

    mc_resp
        .json::<MinecraftAuthResponse>()
        .await
        .map_err(|e| MonoryxError::Auth(format!("Failed to parse Minecraft auth response: {e}")))
}

pub async fn fetch_minecraft_profile(
    http: &reqwest::Client,
    mc_access_token: &str,
) -> Result<MinecraftProfileResponse> {
    let profile_resp = http
        .get("https://api.minecraftservices.com/minecraft/profile")
        .header(reqwest::header::ACCEPT, "application/json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {mc_access_token}"),
        )
        .send()
        .await
        .map_err(|e| MonoryxError::Auth(format!("Failed to fetch Minecraft profile: {e}")))?;

    if !profile_resp.status().is_success() {
        let status = profile_resp.status();
        let err_text = profile_resp.text().await.unwrap_or_default();
        if is_not_found(status, &err_text) {
            return Err(MonoryxError::Auth(
                "This Microsoft account does not own Minecraft Java Edition (or no profile name has been set yet at minecraft.net).".into(),
            ));
        }
        let msg = format_mojang_error_detail(status, &err_text);
        return Err(MonoryxError::Auth(format!(
            "Failed to fetch Minecraft profile: {msg}. Does this Microsoft account own Minecraft Java Edition?"
        )));
    }

    profile_resp
        .json::<MinecraftProfileResponse>()
        .await
        .map_err(|e| MonoryxError::Auth(format!("Failed to parse Minecraft profile response: {e}")))
}

pub(crate) fn is_not_found(status: reqwest::StatusCode, err_text: &str) -> bool {
    if status == reqwest::StatusCode::NOT_FOUND {
        return true;
    }
    let upper = err_text.to_ascii_uppercase();
    upper.contains("NOT_FOUND")
        || upper.contains("NOT FOUND")
        || upper.contains("THE SERVER HAS NOT FOUND ANYTHING MATCHING THE REQUEST-URI")
        || upper.contains("COULD NOT FIND")
        || upper.contains("PROFILE NOT FOUND")
}

pub(crate) fn parse_mojang_error_message(err_text: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(err_text).ok()?;
    let err = v
        .get("error")
        .or_else(|| v.get("errorType"))
        .and_then(|m| m.as_str());
    let msg = v
        .get("errorMessage")
        .or_else(|| v.get("developerMessage"))
        .and_then(|m| m.as_str());
    match (msg, err) {
        (Some(m), Some(e)) if e != m => Some(format!("{m} ({e})")),
        (Some(m), _) => Some(m.to_string()),
        (None, Some(e)) => Some(e.to_string()),
        (None, None) => None,
    }
}

pub(crate) fn format_mojang_error_detail(status: reqwest::StatusCode, err_text: &str) -> String {
    let parsed_message = parse_mojang_error_message(err_text);
    match parsed_message {
        Some(m) if !m.is_empty() => format!("{m} (HTTP {status})"),
        _ if !err_text.trim().is_empty() => format!("{err_text} (HTTP {status})"),
        _ => format!("HTTP {status}"),
    }
}

pub(crate) fn parse_xerr_value(val: &serde_json::Value) -> Option<u64> {
    if let Some(num) = val.as_u64() {
        return Some(num);
    }
    if let Some(s) = val.as_str() {
        let clean = s.trim();
        if let Ok(num) = clean.parse::<u64>() {
            return Some(num);
        }
        let hex_str = clean
            .strip_prefix("0x")
            .or_else(|| clean.strip_prefix("0X"))
            .unwrap_or(clean);
        if let Ok(num) = u64::from_str_radix(hex_str, 16) {
            return Some(num);
        }
    }
    None
}

pub(crate) fn map_xerr_code(xerr: u64) -> Option<&'static str> {
    match xerr {
        2148916233 => Some(
            "This Microsoft account does not have an Xbox profile. Create one at xbox.com first.",
        ),
        2148916234 => Some("Your Xbox Live account has been banned or suspended."),
        2148916235 => {
            Some("Xbox Live is not available in your region, or this account is suspended.")
        }
        2148916236 | 2148916237 => {
            Some("Adult verification required. Please verify your age on xbox.com.")
        }
        2148916238 => {
            Some("This account is a child account and must be added to a Microsoft Family to play.")
        }
        _ => None,
    }
}

pub async fn complete_minecraft_login(
    http: &reqwest::Client,
    ms_access_token: &str,
    ms_refresh_token: &str,
) -> Result<MicrosoftProfile> {
    let xbl_body = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={ms_access_token}")
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let xbl_resp = http
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&xbl_body)
        .send()
        .await
        .map_err(|e| MonoryxError::Auth(format!("Xbox Live authentication request failed: {e}")))?;

    if !xbl_resp.status().is_success() {
        let status = xbl_resp.status();
        let err_text = xbl_resp.text().await.unwrap_or_default();
        let err_json: serde_json::Value = serde_json::from_str(&err_text).unwrap_or_default();
        if let Some(xerr) = err_json.get("XErr").and_then(parse_xerr_value) {
            if let Some(msg) = map_xerr_code(xerr) {
                return Err(MonoryxError::Auth(msg.into()));
            }
        }
        let msg = if !err_text.trim().is_empty() {
            format!("Xbox Live authentication failed (HTTP {status}: {err_text}). Please verify your Microsoft account.")
        } else {
            "Xbox Live authentication failed. Please verify your Microsoft account.".into()
        };
        return Err(MonoryxError::Auth(msg));
    }

    let xbl_data: XboxAuthResponse = xbl_resp
        .json()
        .await
        .map_err(|e| MonoryxError::Auth(format!("Failed to parse Xbox Live response: {e}")))?;

    let xbl_token = xbl_data.token;

    let xsts_body = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let xsts_resp = http
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&xsts_body)
        .send()
        .await
        .map_err(|e| MonoryxError::Auth(format!("XSTS token request failed: {e}")))?;

    if !xsts_resp.status().is_success() {
        let status = xsts_resp.status();
        let err_text = xsts_resp.text().await.unwrap_or_default();
        let err_json: serde_json::Value = serde_json::from_str(&err_text).unwrap_or_default();
        if let Some(xerr) = err_json.get("XErr").and_then(parse_xerr_value) {
            if let Some(msg) = map_xerr_code(xerr) {
                return Err(MonoryxError::Auth(msg.into()));
            }
        }
        let msg = if !err_text.trim().is_empty() {
            format!("XSTS authorization failed (HTTP {status}: {err_text}). Check your Xbox Live account settings.")
        } else {
            "XSTS authorization failed. Check your Xbox Live account settings.".into()
        };
        return Err(MonoryxError::Auth(msg));
    }

    let xsts_data: XboxAuthResponse = xsts_resp
        .json()
        .await
        .map_err(|e| MonoryxError::Auth(format!("Failed to parse XSTS response: {e}")))?;

    let xsts_token = xsts_data.token;
    let uhs = xsts_data
        .display_claims
        .xui
        .first()
        .map(|c| c.uhs.clone())
        .or_else(|| xbl_data.display_claims.xui.first().map(|c| c.uhs.clone()))
        .ok_or_else(|| MonoryxError::Auth("No Xbox user hash (uhs) returned.".into()))?;

    let mc_data = login_with_xbox(http, &uhs, &xsts_token).await?;
    let profile_data = fetch_minecraft_profile(http, &mc_data.access_token).await?;

    let parsed_uuid = parse_undashed_uuid(&profile_data.id)
        .map_err(|e| MonoryxError::Auth(format!("Invalid Minecraft profile UUID: {e}")))?;

    let expires_in_secs = mc_data.expires_in.unwrap_or(86400);
    let expires_at = chrono::Utc::now().timestamp() + (expires_in_secs as i64);

    Ok(MicrosoftProfile {
        username: profile_data.name,
        uuid: parsed_uuid,
        access_token: mc_data.access_token,
        refresh_token: ms_refresh_token.to_string(),
        expires_at,
    })
}

pub async fn authenticate_with_device_code(
    http: &reqwest::Client,
    client_id: &str,
    device_code: &str,
) -> Result<Option<MicrosoftProfile>> {
    if let Some(token) = poll_device_token(http, client_id, device_code).await? {
        let profile = complete_minecraft_login(
            http,
            &token.access_token,
            token.refresh_token.as_deref().unwrap_or(""),
        )
        .await?;
        Ok(Some(profile))
    } else {
        Ok(None)
    }
}

pub fn parse_undashed_uuid(s: &str) -> std::result::Result<uuid::Uuid, String> {
    let clean = s.trim();
    if clean.len() == 32 && clean.is_ascii() {
        let formatted = format!(
            "{}-{}-{}-{}-{}",
            &clean[0..8],
            &clean[8..12],
            &clean[12..16],
            &clean[16..20],
            &clean[20..32]
        );
        uuid::Uuid::parse_str(&formatted).map_err(|e| e.to_string())
    } else {
        uuid::Uuid::parse_str(clean).map_err(|e| e.to_string())
    }
}

pub async fn refresh_minecraft_token(
    http: &reqwest::Client,
    client_id: &str,
    refresh_token: &str,
) -> Result<MicrosoftProfile> {
    let id = if client_id.trim().is_empty() {
        DEFAULT_CLIENT_ID
    } else {
        client_id.trim()
    };
    let params = [
        ("grant_type", "refresh_token"),
        ("client_id", id),
        ("refresh_token", refresh_token),
    ];
    let resp = http
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| MonoryxError::Auth(format!("Failed to refresh Microsoft token: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        return Err(MonoryxError::Auth(format!(
            "Microsoft session expired (HTTP {status}: {err_text}). Please sign in again."
        )));
    }

    let tok: DeviceTokenResponse = resp
        .json()
        .await
        .map_err(|e| MonoryxError::Auth(format!("Failed to parse refreshed token: {e}")))?;

    let new_refresh = tok.refresh_token.as_deref().unwrap_or(refresh_token);
    complete_minecraft_login(http, &tok.access_token, new_refresh).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_undashed_and_dashed_uuid() {
        let undashed = "4566e69fc90748ee8d71d7ba5aa00d20";
        let parsed = parse_undashed_uuid(undashed).unwrap();
        assert_eq!(
            parsed.hyphenated().to_string(),
            "4566e69f-c907-48ee-8d71-d7ba5aa00d20"
        );

        let dashed = "4566e69f-c907-48ee-8d71-d7ba5aa00d20";
        assert_eq!(parse_undashed_uuid(dashed).unwrap(), parsed);
    }

    #[test]
    fn invalid_uuid_errors() {
        assert!(parse_undashed_uuid("not-a-valid-uuid").is_err());
        assert!(parse_undashed_uuid("").is_err());
    }

    #[test]
    fn default_client_id_is_non_empty() {
        assert!(!DEFAULT_CLIENT_ID.is_empty());
    }

    #[test]
    fn identity_token_format_structure() {
        let uhs = "1234567890abcdef";
        let xsts = "mock_xsts_token_payload";
        let payload = serde_json::json!({
            "identityToken": format!("XBL3.0 x={uhs};{xsts}")
        });
        assert_eq!(
            payload["identityToken"],
            "XBL3.0 x=1234567890abcdef;mock_xsts_token_payload"
        );
    }

    #[test]
    fn minecraft_profile_response_deserialization() {
        let json = r#"{"id":"4566e69fc90748ee8d71d7ba5aa00d20","name":"PlayerName"}"#;
        let res: MinecraftProfileResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.id, "4566e69fc90748ee8d71d7ba5aa00d20");
        assert_eq!(res.name, "PlayerName");
    }

    #[test]
    fn parse_mojang_error_variants() {
        assert!(is_not_found(
            reqwest::StatusCode::NOT_FOUND,
            r#"{"error":"NOT_FOUND","errorMessage":"The server has not found anything matching the request-URI"}"#
        ));
        assert!(is_not_found(
            reqwest::StatusCode::BAD_REQUEST,
            "NOT_FOUND in body"
        ));
        assert!(!is_not_found(
            reqwest::StatusCode::FORBIDDEN,
            r#"{"error":"Forbidden","errorMessage":"Invalid app registration"}"#
        ));

        let formatted = parse_mojang_error_message(
            r#"{"error":"Forbidden","errorMessage":"Invalid app registration"}"#,
        );
        assert_eq!(
            formatted.as_deref(),
            Some("Invalid app registration (Forbidden)")
        );

        let detail = format_mojang_error_detail(
            reqwest::StatusCode::FORBIDDEN,
            r#"{"error":"Forbidden","errorMessage":"Invalid app registration"}"#,
        );
        assert_eq!(
            detail,
            "Invalid app registration (Forbidden) (HTTP 403 Forbidden)"
        );
    }

    #[test]
    fn map_xerr_codes() {
        assert!(map_xerr_code(2148916233).unwrap().contains("Xbox profile"));
        assert!(map_xerr_code(2148916234)
            .unwrap()
            .contains("banned or suspended"));
        assert!(map_xerr_code(2148916235).unwrap().contains("suspended"));
        assert!(map_xerr_code(2148916236)
            .unwrap()
            .contains("Adult verification"));
        assert!(map_xerr_code(2148916237)
            .unwrap()
            .contains("Adult verification"));
        assert!(map_xerr_code(2148916238).unwrap().contains("child account"));
        assert!(map_xerr_code(999999999).is_none());
    }

    #[test]
    fn parse_xerr_formats() {
        let int_val = serde_json::json!(2148916238u64);
        assert_eq!(parse_xerr_value(&int_val), Some(2148916238));

        let str_val = serde_json::json!("2148916238");
        assert_eq!(parse_xerr_value(&str_val), Some(2148916238));

        let hex_val = serde_json::json!("0x8015dc0e");
        assert_eq!(parse_xerr_value(&hex_val), Some(2148916238));

        let hex_no_prefix = serde_json::json!("8015DC0E");
        assert_eq!(parse_xerr_value(&hex_no_prefix), Some(2148916238));

        let invalid = serde_json::json!("not_a_number");
        assert_eq!(parse_xerr_value(&invalid), None);
    }
}
