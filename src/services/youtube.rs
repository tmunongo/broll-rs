use crate::models::VideoResult;
use anyhow::Result;
use serde::Deserialize;
use tokio::process::Command;

#[derive(Deserialize)]
struct YtInfo {
    id: String,
    title: Option<String>,
    duration: Option<f64>,
    thumbnail: Option<String>,
    webpage_url: Option<String>,
    thumbnails: Option<Vec<YtThumb>>,
}

#[derive(Deserialize)]
struct YtThumb {
    url: String,
}

pub async fn search(query: &str, count: usize) -> Vec<VideoResult> {
    match do_search(query, count).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("YouTube search failed: {e}");
            vec![]
        }
    }
}

async fn do_search(query: &str, count: usize) -> Result<Vec<VideoResult>> {
    let search_term = format!("ytsearch{count}:{query}");

    let output = Command::new("yt-dlp")
        .args([
            &search_term,
            "--dump-json",
            "--no-playlist",
            "--quiet",
            "--no-warnings",
            "--skip-download",
        ])
        .output()
        .await?;

    if !output.status.success() {
        anyhow::bail!("yt-dlp exited with status {}", output.status);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let results = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| serde_json::from_str::<YtInfo>(line).ok())
        .map(|info| {
            // Pick a mid-quality thumbnail
            let thumbnail = info
                .thumbnails
                .as_ref()
                .and_then(|ts| {
                    let mid = ts.len() / 2;
                    ts.get(mid).map(|t| t.url.clone())
                })
                .or(info.thumbnail);

            let url = info
                .webpage_url
                .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", info.id));

            VideoResult {
                id: format!("youtube_{}", info.id),
                title: info.title.unwrap_or_else(|| "Untitled".into()),
                source: "youtube".into(),
                duration: info.duration,
                thumbnail,
                preview_url: Some(format!("https://www.youtube.com/embed/{}", info.id)),
                download_url: Some(url),
                license: Some("YouTube (check individual video license)".into()),
                width: None,
                height: None,
            }
        })
        .collect();

    Ok(results)
}
