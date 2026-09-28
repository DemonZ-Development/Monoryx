use crate::storage::cache::DiskCache;
use futures::StreamExt as _;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const NEW_FEED: &str = "https://launchercontent.mojang.com/v2/javaPatchNotes.json";
const OLD_FEED: &str = "https://launchercontent.mojang.com/javaPatchNotes.json";

async fn read_limited(response: reqwest::Response, limit: usize) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        return Err("Official changelog response is too large".to_string());
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| error.to_string())?;
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err("Official changelog response is too large".to_string());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchNote {
    pub title: String,
    pub version: String,
    #[serde(default)]
    pub short_text: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub content_path: String,
    #[serde(default)]
    pub date: String,
}

#[derive(Deserialize)]
struct Feed {
    entries: Vec<PatchNote>,
}

async fn get_feed(
    http: &reqwest::Client,
    cache: &DiskCache,
    key: &str,
    url: &str,
) -> Result<Feed, String> {
    let bytes = if let Some(bytes) = cache.get(key) {
        bytes
    } else {
        let response = http
            .get(url)
            .send()
            .await
            .map_err(|error| error.to_string());
        match response {
            Ok(response) if response.status().is_success() => {
                let bytes = read_limited(response, 32_000_000).await?;
                let _ = cache.put(key, &bytes);
                bytes
            }
            _ => cache
                .get_stale(key)
                .ok_or_else(|| "Patch notes are unavailable offline".to_string())?,
        }
    };
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

pub async fn load_index(
    http: &reqwest::Client,
    cache: &DiskCache,
) -> Result<HashMap<String, PatchNote>, String> {
    let (old, new) = tokio::join!(
        get_feed(http, cache, "java-patch-notes-v1", OLD_FEED),
        get_feed(http, cache, "java-patch-notes-v2", NEW_FEED)
    );
    if old.is_err() && new.is_err() {
        return Err("Official Minecraft patch notes are unavailable right now".to_string());
    }
    let mut notes = HashMap::new();
    for feed in [old, new].into_iter().flatten() {
        for note in feed.entries {
            if !note.version.is_empty() {
                notes.insert(note.version.clone(), note);
            }
        }
    }
    Ok(notes)
}

pub async fn load_full(
    http: &reqwest::Client,
    cache: &DiskCache,
    note: &PatchNote,
) -> Result<String, String> {
    if !note.body.is_empty() {
        return Ok(html_to_text(&note.body));
    }
    let path = note.content_path.as_str();
    if !path.starts_with("javaPatchNotes/")
        || !path.ends_with(".json")
        || path.contains("..")
        || path.contains('\\')
    {
        return Ok(note.short_text.clone());
    }
    let key = format!("java-patch-note-{}", note.version);
    let bytes = if let Some(bytes) = cache.get(&key) {
        bytes
    } else {
        let url = format!("https://launchercontent.mojang.com/v2/{path}");
        match http.get(url).send().await {
            Ok(response) if response.status().is_success() => {
                let bytes = read_limited(response, 4_000_000).await?;
                let _ = cache.put(&key, &bytes);
                bytes
            }
            _ => cache.get_stale(&key).unwrap_or_default(),
        }
    };
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    let body = value
        .get("body")
        .or_else(|| value.get("content"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if body.is_empty() {
        Ok(note.short_text.clone())
    } else {
        Ok(html_to_text(body))
    }
}

fn html_to_text(html: &str) -> String {
    let mut result = String::new();
    let mut tag = String::new();
    let mut inside = false;
    let mut chars = html.chars();
    while let Some(ch) = chars.next() {
        if ch == '<' {
            inside = true;
            tag.clear();
            continue;
        }
        if inside {
            if ch == '>' {
                inside = false;
                let name = tag.trim().to_ascii_lowercase();
                if name.starts_with("br")
                    || name.starts_with("/p")
                    || name.starts_with("/h")
                    || name.starts_with("/li")
                    || name.starts_with("/ul")
                {
                    if !result.ends_with('\n') {
                        result.push('\n');
                    }
                } else if name.starts_with("li") {
                    result.push_str("• ");
                }
            } else {
                tag.push(ch);
            }
            continue;
        }
        if ch == '&' {
            let mut entity = String::new();
            for next in chars.by_ref().take(12) {
                if next == ';' {
                    break;
                }
                entity.push(next);
            }
            match entity.as_str() {
                "amp" => result.push('&'),
                "lt" => result.push('<'),
                "gt" => result.push('>'),
                "quot" => result.push('"'),
                "apos" | "#39" => result.push('\''),
                "nbsp" => result.push(' '),
                _ => {
                    if let Some(number) = entity
                        .strip_prefix("#x")
                        .and_then(|digits| u32::from_str_radix(digits, 16).ok())
                        .or_else(|| {
                            entity
                                .strip_prefix('#')
                                .and_then(|digits| digits.parse::<u32>().ok())
                        })
                        .and_then(char::from_u32)
                    {
                        result.push(number);
                    } else {
                        result.push('&');
                        result.push_str(&entity);
                        result.push(';');
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn official_search_url(version: &str) -> String {
    let query = format!("Minecraft Java Edition {version}");
    let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    format!("https://feedback.minecraft.net/hc/en-us/search?query={encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notes_preserve_text_and_links_are_encoded() {
        assert_eq!(
            html_to_text("<h1>Changes</h1><ul><li>World &amp; blocks</li></ul>"),
            "Changes\n• World & blocks"
        );
        assert!(official_search_url("26.3-snapshot-1").contains("26.3-snapshot-1"));
    }
}
