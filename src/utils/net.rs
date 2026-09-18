use crate::error::{MonoryxError, Result};
use std::time::Duration;

pub const MONORYX_USER_AGENT: &str = concat!("monoryx/", env!("CARGO_PKG_VERSION"));

pub fn modrinth_user_agent() -> String {
    format!(
        "monoryx/{} (https://github.com/monoryx/monoryx)",
        env!("CARGO_PKG_VERSION")
    )
}

pub fn create_client() -> Result<reqwest::Client> {
    let client = reqwest::Client::builder()
        .user_agent(MONORYX_USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .pool_max_idle_per_host(8)
        .build()?;
    Ok(client)
}

pub async fn get_json_with_retry<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    extra_ua: Option<&str>,
) -> Result<T> {
    let response = get_with_retry(client, url, extra_ua).await?;
    Ok(response.json::<T>().await?)
}

pub async fn get_text_with_retry(client: &reqwest::Client, url: &str) -> Result<String> {
    let response = get_with_retry(client, url, None).await?;
    Ok(response.text().await?)
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
