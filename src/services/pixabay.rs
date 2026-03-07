use anyhow::Result;
use serde::Deserialize;
use crate::models::VideoResult;

#[derive(Deserialize)]
struct PixabayResponse {
    hits: Vec<PixabayHit>,
}

#[derive(Deserialize)]
struct PixabayHit {
    id: u64,
    duration: Option<f64>,
    #[serde(rename = "userImageURL")]
    user_image_url: Option<String>,
    videos: PixabayVideos,
}

#[derive(Deserialize)]
struct PixabayVideos {
    large: Option<PixabayFile>,
    medium: Option<PixabayFile>,
    small: Option<PixabayFile>,
    tiny: Option<PixabayFile>,
}

#[derive(Deserialize, Clone)]
struct PixabayFile {
    url: String,
    width: Option<u32>,
    height: Option<u32>,
}

impl PixabayVideos {
    fn best(self) -> Option<PixabayFile> {
        self.large.or(self.medium).or(self.small).or(self.tiny)
    }
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
            tracing::warn!("Pixabay search failed: {e}");
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
    let resp: PixabayResponse = client
        .get("https://pixabay.com/api/videos/")
        .query(&[
            ("key", api_key),
            ("q", query),
            ("per_page", &per_page.to_string()),
            ("video_type", "film"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let results = resp
        .hits
        .into_iter()
        .filter_map(|hit| {
            let best = hit.videos.best()?;
            Some(VideoResult {
                id: format!("pixabay_{}", hit.id),
                title: format!("{} — Pixabay #{}", titlecase(query), hit.id),
                source: "pixabay".into(),
                duration: hit.duration,
                thumbnail: hit.user_image_url,
                preview_url: Some(best.url.clone()),
                download_url: Some(best.url),
                license: Some("Pixabay License (free commercial use)".into()),
                width: best.width,
                height: best.height,
            })
        })
        .collect();

    Ok(results)
}

fn titlecase(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
