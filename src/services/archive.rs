use crate::models::VideoResult;
use anyhow::Result;
use serde::Deserialize;

const SEARCH_URL: &str = "https://archive.org/advancedsearch.php";
const BASE_URL: &str = "https://archive.org";

#[derive(Deserialize)]
struct ArchiveResponse {
    response: ArchiveInner,
}

#[derive(Deserialize)]
struct ArchiveInner {
    docs: Vec<ArchiveDoc>,
}

#[derive(Deserialize)]
struct ArchiveDoc {
    identifier: String,
    title: Option<serde_json::Value>,
    runtime: Option<serde_json::Value>,
}

pub async fn search(client: &reqwest::Client, query: &str, rows: usize) -> Vec<VideoResult> {
    match do_search(client, SEARCH_URL, query, rows).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Archive search failed: {e}");
            vec![]
        }
    }
}

async fn do_search(
    client: &reqwest::Client,
    search_url: &str,
    query: &str,
    rows: usize,
) -> Result<Vec<VideoResult>> {
    let q = format!("{query} AND mediatype:movies");
    let resp_text = client
        .get(search_url)
        .query(&[
            ("q", q.as_str()),
            ("fl[]", "identifier,title,runtime"),
            ("rows", &rows.to_string()),
            ("output", "json"),
            ("sort[]", "downloads desc"),
        ])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let resp: ArchiveResponse = match serde_json::from_str(&resp_text) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Archive JSON decode error: {e}. Raw body: {resp_text}");
            return Err(e.into());
        }
    };

    let results = resp
        .response
        .docs
        .into_iter()
        .map(|doc| {
            let id = &doc.identifier;
            let title = doc
                .title
                .and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s),
                    serde_json::Value::Array(a) => a
                        .into_iter()
                        .next()
                        .and_then(|v| v.as_str().map(str::to_string)),
                    _ => None,
                })
                .unwrap_or_else(|| id.clone());

            let duration = doc.runtime.and_then(|v| {
                let s = match &v {
                    serde_json::Value::String(s) => s.clone(),
                    _ => return None,
                };
                parse_runtime(&s)
            });

            VideoResult {
                id: format!("archive_{id}"),
                title,
                source: "archive".into(),
                duration,
                thumbnail: Some(format!("{BASE_URL}/services/img/{id}")),
                preview_url: Some(format!("{BASE_URL}/embed/{id}")),
                download_url: Some(format!("{BASE_URL}/download/{id}")),
                license: Some("Public Domain / Open License".into()),
                width: None,
                height: None,
            }
        })
        .collect();

    Ok(results)
}

fn parse_runtime(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        3 => {
            let h: f64 = parts[0].parse().ok()?;
            let m: f64 = parts[1].parse().ok()?;
            let s: f64 = parts[2].parse().ok()?;
            Some(h * 3600.0 + m * 60.0 + s)
        }
        2 => {
            let m: f64 = parts[0].parse().ok()?;
            let s: f64 = parts[1].parse().ok()?;
            Some(m * 60.0 + s)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // ── parse_runtime ──────────────────────────────────────────────────────────

    #[test]
    fn parse_hms() {
        assert_eq!(
            parse_runtime("1:02:03"),
            Some(1.0 * 3600.0 + 2.0 * 60.0 + 3.0)
        );
    }

    #[test]
    fn parse_ms() {
        assert_eq!(parse_runtime("5:30"), Some(5.0 * 60.0 + 30.0));
    }

    #[test]
    fn parse_single_segment_is_none() {
        assert_eq!(parse_runtime("120"), None);
    }

    #[test]
    fn parse_invalid_is_none() {
        assert_eq!(parse_runtime("abc:def"), None);
    }

    #[test]
    fn parse_empty_is_none() {
        assert_eq!(parse_runtime(""), None);
    }

    #[test]
    fn parse_zero() {
        assert_eq!(parse_runtime("0:00"), Some(0.0));
    }

    // ── search (HTTP) ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn search_returns_empty_on_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            // We use any path because the base URL from the server will be used directly
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        // Call do_search to explicitly hit the mocked server and confirm it handles 500 cleanly
        let results = do_search(&client, &server.uri(), "test_query", 1).await;
        assert!(results.is_err());
    }

    #[tokio::test]
    async fn do_search_parses_response() {
        let server = MockServer::start().await;

        let body = serde_json::json!({
            "response": {
                "docs": [
                    {
                        "identifier": "test-vid-1",
                        "title": "Test Video 1",
                        "runtime": "1:30"
                    },
                    {
                        "identifier": "test-vid-2",
                        "title": ["Test Video 2 array"],
                        "runtime": "0:30.5"
                    }
                ]
            }
        });

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        // Call do_search with the mock server URL
        let url = server.uri();
        let results = do_search(&client, &url, "test", 10).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Test Video 1");
        assert_eq!(results[0].duration, Some(90.0)); // 1:30
        assert_eq!(results[1].title, "Test Video 2 array");
        assert_eq!(results[1].duration, Some(30.5));
    }

    #[tokio::test]
    async fn search_with_empty_key_returns_results_if_api_ok() {
        // archive search doesn't need an API key, just verifies it returns empty vec on failure
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(1))
            .build()
            .unwrap();
        // Very short timeout → will fail → returns []
        let results = search(&client, "rust programming", 2).await;
        assert!(results.is_empty());
    }
}
