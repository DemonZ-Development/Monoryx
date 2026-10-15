use crate::error::{MonoryxError, Result};
use futures::StreamExt as _;
use std::time::Duration;

pub const MONORYX_USER_AGENT: &str = concat!("monoryx/", env!("CARGO_PKG_VERSION"));

pub fn modrinth_user_agent() -> String {
    format!(
        "monoryx/{} ({})",
        env!("CARGO_PKG_VERSION"),
        crate::utils::links::PROJECT_URL
    )
}

pub fn create_client() -> Result<reqwest::Client> {
    let client = reqwest::Client::builder()
        .user_agent(MONORYX_USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .pool_max_idle_per_host(4)
        .pool_idle_timeout(Duration::from_secs(90))
        .build()?;
    Ok(client)
}

pub const MAX_TEXT_BODY_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_JSON_BODY_BYTES: usize = 32 * 1024 * 1024;

async fn read_capped_bytes(response: reqwest::Response, limit: usize) -> Result<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(MonoryxError::Download(format!(
            "Response body exceeded the {limit} byte limit"
        )));
    }
    let capacity = response
        .content_length()
        .and_then(|length| usize::try_from(length).ok())
        .unwrap_or(0);
    let mut body = Vec::with_capacity(capacity);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if chunk.len() > limit.saturating_sub(body.len()) {
            return Err(MonoryxError::Download(format!(
                "Response body exceeded the {limit} byte limit"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub async fn read_capped_text(response: reqwest::Response, limit: usize) -> Result<String> {
    String::from_utf8(read_capped_bytes(response, limit).await?)
        .map_err(|_| MonoryxError::Download("Response body was not valid UTF-8".to_string()))
}

pub async fn get_json_with_retry<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    extra_ua: Option<&str>,
) -> Result<T> {
    let response = get_with_retry(client, url, extra_ua).await?;
    if let Some(len) = response.content_length() {
        if len > MAX_JSON_BODY_BYTES as u64 {
            return Err(MonoryxError::Download(format!(
                "Refusing a {len} byte metadata response from {url}"
            )));
        }
    }
    Ok(serde_json::from_slice(
        &read_capped_bytes(response, MAX_JSON_BODY_BYTES).await?,
    )?)
}

pub async fn get_text_with_retry(client: &reqwest::Client, url: &str) -> Result<String> {
    let response = get_with_retry(client, url, None).await?;
    read_capped_text(response, MAX_TEXT_BODY_BYTES).await
}

async fn get_with_retry(
    client: &reqwest::Client,
    url: &str,
    extra_ua: Option<&str>,
) -> Result<reqwest::Response> {
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        let mut req = client.get(url);
        if let Some(ua) = extra_ua {
            req = req.header(reqwest::header::USER_AGENT, ua);
        }
        let resp = req.send().await;
        match resp {
            Err(e) if attempt < 4 && (e.is_timeout() || e.is_connect()) => {
                tokio::time::sleep(Duration::from_millis(300 * u64::from(attempt))).await;
                continue;
            }
            Err(e) => return Err(MonoryxError::Http(e)),
            Ok(r) => {
                let status = r.status();
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS && attempt < 5 {
                    let wait = r
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(attempt as u64);
                    tokio::time::sleep(Duration::from_secs(wait.min(10))).await;
                    continue;
                }
                if status.is_server_error() && attempt < 4 {
                    tokio::time::sleep(Duration::from_millis(500 * u64::from(attempt))).await;
                    continue;
                }
                if !status.is_success() {
                    return Err(MonoryxError::Download(format!(
                        "GET {url} returned HTTP {status}"
                    )));
                }
                return Ok(r);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn serve(
        responses: Vec<(&'static str, &'static str)>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/metadata", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buffer = [0; 4096];
                let mut received = 0;
                loop {
                    assert!(
                        received < buffer.len(),
                        "request headers exceeded test buffer"
                    );
                    let count = stream.read(&mut buffer[received..]).await.unwrap();
                    assert!(count > 0, "connection closed before request headers");
                    received += count;
                    if buffer[..received]
                        .windows(4)
                        .any(|bytes| bytes == b"\r\n\r\n")
                    {
                        break;
                    }
                }
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        });
        (url, task)
    }

    async fn serve_streamed(chunked: bool, size: usize) -> (String, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/metadata", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 4096];
            let mut received = 0;
            loop {
                assert!(received < buffer.len());
                let count = stream.read(&mut buffer[received..]).await.unwrap();
                assert!(count > 0);
                received += count;
                if buffer[..received]
                    .windows(4)
                    .any(|bytes| bytes == b"\r\n\r\n")
                {
                    break;
                }
            }
            let headers = if chunked {
                "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
            } else {
                "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n"
            };
            stream.write_all(headers.as_bytes()).await.unwrap();
            let chunk = vec![b' '; 64 * 1024];
            let mut remaining = size;
            while remaining > 0 {
                let length = remaining.min(chunk.len());
                if chunked
                    && stream
                        .write_all(format!("{length:x}\r\n").as_bytes())
                        .await
                        .is_err()
                {
                    return;
                }
                if stream.write_all(&chunk[..length]).await.is_err() {
                    return;
                }
                if chunked && stream.write_all(b"\r\n").await.is_err() {
                    return;
                }
                remaining -= length;
            }
            if chunked {
                let _ = stream.write_all(b"0\r\n\r\n").await;
            }
        });
        (url, task)
    }

    #[tokio::test]
    async fn text_caps_actual_bytes_with_chunked_or_unknown_length_responses() {
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        for chunked in [true, false] {
            let (url, task) = serve_streamed(chunked, 17).await;
            let response = client.get(&url).send().await.unwrap();
            assert_eq!(response.content_length(), None);
            assert!(matches!(read_capped_text(response, 16).await,
                Err(MonoryxError::Download(message)) if message.contains("16 byte limit")));
            task.await.unwrap();
            let (url, task) = serve_streamed(chunked, 16).await;
            let response = client.get(&url).send().await.unwrap();
            assert_eq!(
                read_capped_text(response, 16).await.unwrap(),
                " ".repeat(16)
            );
            task.await.unwrap();
        }
    }

    #[tokio::test]
    async fn json_caps_actual_bytes_with_chunked_or_unknown_length_responses() {
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        for chunked in [true, false] {
            let (url, task) = serve_streamed(chunked, MAX_JSON_BODY_BYTES + 1).await;
            let result = tokio::time::timeout(
                Duration::from_secs(10),
                get_json_with_retry::<serde_json::Value>(&client, &url, None),
            )
            .await
            .unwrap();
            assert!(matches!(result,
                Err(MonoryxError::Download(message)) if message.contains("byte limit")));
            task.await.unwrap();
        }
    }

    #[tokio::test]
    async fn xml_http_errors_are_not_treated_as_empty_metadata() {
        let (url, task) = serve(vec![("404 Not Found", "<metadata/>")]).await;
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        assert!(matches!(get_text_with_retry(&client, &url).await,
            Err(MonoryxError::Download(message)) if message.contains("404")));
        task.await.unwrap();
    }

    #[tokio::test]
    async fn xml_retries_server_errors_before_reading_metadata() {
        let (url, task) = serve(vec![
            ("503 Service Unavailable", "unavailable"),
            ("200 OK", "<metadata/>"),
        ])
        .await;
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        assert_eq!(
            get_text_with_retry(&client, &url).await.unwrap(),
            "<metadata/>"
        );
        task.await.unwrap();
    }

    #[tokio::test]
    async fn json_still_checks_status_and_deserializes() {
        let (url, task) = serve(vec![("200 OK", r#"{"version":"1.21.1"}"#)]).await;
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let value: serde_json::Value = get_json_with_retry(&client, &url, None).await.unwrap();
        assert_eq!(value["version"], "1.21.1");
        task.await.unwrap();
        let (url, task) = serve(vec![("404 Not Found", "[]")]).await;
        assert!(get_json_with_retry::<Vec<String>>(&client, &url, None)
            .await
            .is_err());
        task.await.unwrap();
    }
}
