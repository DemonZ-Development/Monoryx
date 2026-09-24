use serde::Deserialize;

const API: &str = "https://game.nexeu.zip/api";

#[derive(Debug, Clone, Default)]
pub struct Session {
    pub generation: u64,
    pub api_key: String,
    pub loading: bool,
    pub error: String,
    pub overview: Option<Overview>,
    pub selected_server: Option<String>,
    pub resources: Option<serde_json::Value>,
    pub logs: Option<String>,
    pub backups: Option<Vec<Backup>>,
    pub console_command: String,
}

#[derive(Debug, Clone)]
pub struct Overview {
    pub account: serde_json::Value,
    pub servers: Vec<Server>,
    pub announcements: Vec<Announcement>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    pub uuid: String,
    pub uuid_short: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub allocation: Option<Allocation>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Allocation {
    pub ip: Option<String>,
    pub ip_alias: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Announcement {
    pub title: String,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Backup {
    pub name: String,
    pub is_successful: Option<bool>,
    pub bytes: Option<u64>,
    pub created: Option<String>,
}

#[derive(Deserialize)]
struct BackupsResponse {
    backups: BackupPage,
}

#[derive(Deserialize)]
struct BackupPage {
    data: Vec<Backup>,
}

#[derive(Deserialize)]
struct ServersResponse {
    servers: ServerPage,
}

#[derive(Deserialize)]
struct ServerPage {
    total: usize,
    page: usize,
    data: Vec<Server>,
}

#[derive(Deserialize)]
struct AnnouncementsResponse {
    announcements: Vec<Announcement>,
}

fn server_id(id: &str) -> Result<&str, String> {
    if (8..=36).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
        Ok(id)
    } else {
        Err("invalid server identifier".to_string())
    }
}

fn auth_header(key: &str) -> String {
    let key = key.trim();
    if key.starts_with("Bearer ") {
        key.to_string()
    } else {
        format!("Bearer {key}")
    }
}

async fn get<T: serde::de::DeserializeOwned>(
    http: &reqwest::Client,
    key: &str,
    path: &str,
) -> Result<T, String> {
    let response = http
        .get(format!("{API}{path}"))
        .header(reqwest::header::AUTHORIZATION, auth_header(key))
        .send()
        .await
        .map_err(|e| format!("Nexeu connection failed: {e}"))?;
    if !response.status().is_success() {
        return Err(api_error(response.status()));
    }
    response
        .json()
        .await
        .map_err(|e| format!("Nexeu response could not be read: {e}"))
}

fn api_error(status: reqwest::StatusCode) -> String {
    match status {
        reqwest::StatusCode::UNAUTHORIZED => "Nexeu rejected this API key (401). Check that you pasted the full key, and that it is active and allowed from your IP address.".to_string(),
        reqwest::StatusCode::FORBIDDEN => "Nexeu denied access (403). Check the API key permissions for this server action.".to_string(),
        _ => format!("Nexeu returned HTTP {status}"),
    }
}

pub async fn overview(http: &reqwest::Client, key: &str) -> Result<Overview, String> {
    let account = get(http, key, "/client/account").await?;
    let mut servers = Vec::new();
    for page in 1..=20 {
        let response: ServersResponse = get(
            http,
            key,
            &format!("/client/servers?page={page}&per_page=50"),
        )
        .await?;
        servers.extend(response.servers.data);
        if servers.len() >= response.servers.total || response.servers.page != page {
            break;
        }
    }
    let announcements = get::<AnnouncementsResponse>(http, key, "/announcements")
        .await
        .map(|r| r.announcements)
        .unwrap_or_default();
    Ok(Overview {
        account,
        servers,
        announcements,
    })
}

pub async fn resources(
    http: &reqwest::Client,
    key: &str,
    id: &str,
) -> Result<serde_json::Value, String> {
    let id = server_id(id)?;
    let response: serde_json::Value =
        get(http, key, &format!("/client/servers/{id}/resources")).await?;
    Ok(response.get("resources").cloned().unwrap_or(response))
}

pub async fn logs(http: &reqwest::Client, key: &str, id: &str) -> Result<String, String> {
    let id = server_id(id)?;
    let response = http
        .get(format!("{API}/client/servers/{id}/logs"))
        .header(reqwest::header::AUTHORIZATION, auth_header(key))
        .send()
        .await
        .map_err(|e| format!("Nexeu connection failed: {e}"))?;
    if !response.status().is_success() {
        return Err(api_error(response.status()));
    }
    response
        .text()
        .await
        .map_err(|e| format!("Nexeu logs could not be read: {e}"))
}

pub async fn backups(http: &reqwest::Client, key: &str, id: &str) -> Result<Vec<Backup>, String> {
    let id = server_id(id)?;
    let response: BackupsResponse = get(
        http,
        key,
        &format!("/client/servers/{id}/backups?page=1&per_page=20&ungrouped=false"),
    )
    .await?;
    Ok(response.backups.data)
}

pub async fn create_backup(http: &reqwest::Client, key: &str, id: &str) -> Result<(), String> {
    let id = server_id(id)?;
    let response = http
        .post(format!("{API}/client/servers/{id}/backups"))
        .header(reqwest::header::AUTHORIZATION, auth_header(key))
        .json(&serde_json::json!({
            "name": null, "backup_group_uuid": null, "database_instance_uuid": null,
            "ignored_files": []
        }))
        .send()
        .await
        .map_err(|e| format!("Nexeu connection failed: {e}"))?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(api_error(response.status()))
    }
}

pub async fn command(
    http: &reqwest::Client,
    key: &str,
    id: &str,
    command: &str,
) -> Result<(), String> {
    let id = server_id(id)?;
    let command = command.trim();
    if command.is_empty() || command.len() > 1024 || command.chars().any(char::is_control) {
        return Err("enter one console command (up to 1024 characters)".to_string());
    }
    let response = http
        .post(format!("{API}/client/servers/{id}/command"))
        .header(reqwest::header::AUTHORIZATION, auth_header(key))
        .json(&serde_json::json!({ "command": command }))
        .send()
        .await
        .map_err(|e| format!("Nexeu connection failed: {e}"))?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(api_error(response.status()))
    }
}

pub async fn power(
    http: &reqwest::Client,
    key: &str,
    id: &str,
    action: &str,
) -> Result<(), String> {
    let id = server_id(id)?;
    if !matches!(action, "start" | "stop" | "restart" | "kill") {
        return Err("invalid power action".to_string());
    }
    let response = http
        .post(format!("{API}/client/servers/{id}/power"))
        .header(reqwest::header::AUTHORIZATION, auth_header(key))
        .json(&serde_json::json!({ "action": action }))
        .send()
        .await
        .map_err(|e| format!("Nexeu connection failed: {e}"))?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(api_error(response.status()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_documented_power_actions_and_safe_ids() {
        assert!(server_id("01234567-89ab-cdef-0123-456789abcdef").is_ok());
        assert!(server_id("../account").is_err());
        assert!(server_id("a?x=1aaa").is_err());
    }

    #[test]
    fn server_page_shape_matches_panel_schema() {
        let response: ServersResponse = serde_json::from_value(serde_json::json!({
            "servers": {"total": 1, "page": 1, "per_page": 10, "data": [{
                "uuid": "01234567-89ab-cdef-0123-456789abcdef", "name": "Survival",
                "allocation": {"ip": "127.0.0.1", "port": 25565}
            }]}
        }))
        .unwrap();
        assert_eq!(response.servers.data[0].name, "Survival");
    }

    #[test]
    fn backup_page_shape_matches_panel_schema() {
        let response: BackupsResponse = serde_json::from_value(serde_json::json!({
            "backups": {"total": 1, "page": 1, "per_page": 10, "data": [{
                "name": "Before update", "is_successful": true, "bytes": 1024
            }]}
        }))
        .unwrap();
        assert_eq!(response.backups.data[0].name, "Before update");
    }

    #[tokio::test]
    async fn empty_console_command_is_rejected_before_network() {
        let http = reqwest::Client::new();
        assert!(
            command(&http, "token", "01234567-89ab-cdef-0123-456789abcdef", "  ")
                .await
                .is_err()
        );
    }
}
