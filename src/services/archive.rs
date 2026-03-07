use anyhow::Result;
use serde::Deserialize;
use crate::models::VideoResult;

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
    match do_search(client, query, rows).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Archive search failed: {e}");
            vec![]
        }
    }
}

async fn do_search(
    client: &reqwest::Client,
    query: &str,
    rows: usize,
) -> Result<Vec<VideoResult>> {
    let q = format!("{query} AND mediatype:movies");
    let resp: ArchiveResponse = client
        .get(SEARCH_URL)
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
        .json()
        .await?;

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
                    serde_json::Value::Array(a) => {
                        a.into_iter().next().and_then(|v| v.as_str().map(str::to_string))
                    }
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
