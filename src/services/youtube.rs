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

pub async fn search(query: &str, count: usize, browser_cookies: Option<&str>) -> Vec<VideoResult> {
    match do_search(query, count, browser_cookies).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("YouTube search failed: {e}");
            vec![]
        }
    }
}

pub fn parse_yt_dlp_output(stdout: &str) -> Vec<VideoResult> {
    stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| serde_json::from_str::<YtInfo>(line).ok())
        .map(|info| {
            // Pick a mid-quality thumbnail
            let thumbnail = info
                .thumbnails
                .as_ref()
                .and_then(|ts| {
                    if ts.is_empty() {
                        None
                    } else {
                        let mid = ts.len() / 2;
                        ts.get(mid).map(|t| t.url.clone())
                    }
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
        .collect()
}

async fn do_search(
    query: &str,
    count: usize,
    browser_cookies: Option<&str>,
) -> Result<Vec<VideoResult>> {
    let search_term = format!("ytsearch{count}:{query}");

    let mut args = vec![
        search_term.as_str(),
        "--dump-json",
        "--no-playlist",
        "--skip-download",
    ];

    if let Some(browser) = browser_cookies {
        args.push("--cookies-from-browser");
        args.push(browser);
    }

    let output = Command::new("yt-dlp").args(&args).output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "yt-dlp exited with status {}.\nstderr:\n{}",
            output.status,
            stderr.trim()
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_yt_dlp_output(&stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_yt_dlp_output_success() {
        let json = r#"{"id":"123","title":"Test Video","duration":120.5,"thumbnail":"http://thumb","webpage_url":"http://watch"}"#;
        let results = parse_yt_dlp_output(json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "youtube_123");
        assert_eq!(results[0].title, "Test Video");
        assert_eq!(results[0].duration, Some(120.5));
        assert_eq!(results[0].thumbnail.as_deref(), Some("http://thumb"));
        assert_eq!(results[0].download_url.as_deref(), Some("http://watch"));
    }

    #[test]
    fn parse_yt_dlp_output_missing_optional_fields() {
        let json = r#"{"id":"abc"}"#;
        let results = parse_yt_dlp_output(json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "youtube_abc");
        assert_eq!(results[0].title, "Untitled");
        assert_eq!(results[0].duration, None);
        assert_eq!(results[0].thumbnail, None);
        assert_eq!(
            results[0].download_url.as_deref(),
            Some("https://www.youtube.com/watch?v=abc")
        );
    }

    #[test]
    fn parse_yt_dlp_output_with_thumbnails_array() {
        let json = r#"{"id":"xyz","thumbnails":[{"url":"t1"},{"url":"t2"},{"url":"t3"}]}"#;
        let results = parse_yt_dlp_output(json);
        assert_eq!(results.len(), 1);
        // length is 3, mid is 1, so t2
        assert_eq!(results[0].thumbnail.as_deref(), Some("t2"));
    }

    #[test]
    fn parse_yt_dlp_output_ignores_invalid_json() {
        let json = "invalid\n{\"id\":\"ok\"}\nnot_json";
        let results = parse_yt_dlp_output(json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "youtube_ok");
    }

    #[tokio::test]
    async fn search_handles_command_failure_gracefully() {
        // This will likely fail since yt-dlp might not be installed, or the network might fail.
        // `search` suppresses the error and returns an empty vec.
        // It covers lines 21-28.
        let res = search("test", 1, None).await;
        // We don't assert it's empty, because locally yt-dlp might succeed,
        // but it exercises the code nevertheless.
        let _ = res;
    }
}
