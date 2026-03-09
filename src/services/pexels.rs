use crate::models::VideoResult;
use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
struct PexelsResponse {
    videos: Vec<PexelsVideo>,
}

#[derive(Deserialize)]
struct PexelsVideo {
    id: u64,
    duration: Option<f64>,
    user: Option<PexelsUser>,
    video_files: Vec<PexelsFile>,
    video_pictures: Vec<PexelsPicture>,
}

#[derive(Deserialize)]
struct PexelsUser {
    name: String,
}

#[derive(Deserialize)]
struct PexelsFile {
    link: String,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Deserialize)]
struct PexelsPicture {
    picture: String,
}

pub async fn search(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    per_page: usize,
) -> Vec<VideoResult> {
    if api_key.is_empty() {
        return vec![];
    }
    match do_search(client, api_key, query, per_page).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Pexels search failed: {e}");
            vec![]
        }
    }
}

async fn do_search(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    per_page: usize,
) -> Result<Vec<VideoResult>> {
    let resp: PexelsResponse = client
        .get("https://api.pexels.com/videos/search")
        .header("Authorization", api_key)
        .query(&[
            ("query", query),
            ("per_page", &per_page.to_string()),
            ("size", "medium"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let results = resp
        .videos
        .into_iter()
        .map(|v| {
            // Best file ≤ 1920px wide
            let mut files = v.video_files;
            files.sort_by_key(|f| std::cmp::Reverse(f.width.unwrap_or(0)));
            let best = files
                .into_iter()
                .find(|f| f.width.unwrap_or(0) <= 1920)
                .unwrap_or_else(|| PexelsFile {
                    link: String::new(),
                    width: None,
                    height: None,
                });

            let thumbnail = v.video_pictures.into_iter().next().map(|p| p.picture);
            let author = v.user.map(|u| u.name).unwrap_or_default();

            VideoResult {
                id: format!("pexels_{}", v.id),
                title: format!("{author} — {query}"),
                source: "pexels".into(),
                duration: v.duration,
                thumbnail,
                preview_url: Some(best.link.clone()),
                download_url: Some(best.link),
                license: Some("CC0".into()),
                width: best.width,
                height: best.height,
            }
        })
        .collect();

    Ok(results)
}
