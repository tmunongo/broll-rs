use crate::models::VideoResult;
use anyhow::Result;
use serde::Deserialize;

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
    match do_search(client, "https://pixabay.com/api/videos/", api_key, query, per_page).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Pixabay search failed: {e}");
            vec![]
        }
    }
}

async fn do_search(
    client: &reqwest::Client,
    search_url: &str,
    api_key: &str,
    query: &str,
    per_page: usize,
) -> Result<Vec<VideoResult>> {
    let resp: PixabayResponse = client
        .get(search_url)
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── titlecase ─────────────────────────────────────────────────────────────

    #[test]
    fn titlecase_basic() {
        assert_eq!(titlecase("hello world"), "Hello world");
    }

    #[test]
    fn titlecase_already_upper() {
        assert_eq!(titlecase("Hello"), "Hello");
    }

    #[test]
    fn titlecase_empty() {
        assert_eq!(titlecase(""), "");
    }

    #[test]
    fn titlecase_single_char() {
        assert_eq!(titlecase("x"), "X");
    }

    // ── PixabayVideos::best ────────────────────────────────────────────────────

    #[test]
    fn best_prefers_large() {
        let vids = PixabayVideos {
            large: Some(PixabayFile { url: "large.mp4".into(), width: Some(1920), height: Some(1080) }),
            medium: Some(PixabayFile { url: "medium.mp4".into(), width: Some(1280), height: Some(720) }),
            small: None,
            tiny: None,
        };
        assert_eq!(vids.best().unwrap().url, "large.mp4");
    }

    #[test]
    fn best_falls_back_to_medium() {
        let vids = PixabayVideos {
            large: None,
            medium: Some(PixabayFile { url: "medium.mp4".into(), width: None, height: None }),
            small: None,
            tiny: None,
        };
        assert_eq!(vids.best().unwrap().url, "medium.mp4");
    }

    #[test]
    fn best_falls_back_to_tiny() {
        let vids = PixabayVideos {
            large: None,
            medium: None,
            small: None,
            tiny: Some(PixabayFile { url: "tiny.mp4".into(), width: None, height: None }),
        };
        assert_eq!(vids.best().unwrap().url, "tiny.mp4");
    }

    #[test]
    fn best_returns_none_when_all_none() {
        let vids = PixabayVideos {
            large: None,
            medium: None,
            small: None,
            tiny: None,
        };
        assert!(vids.best().is_none());
    }

    // ── search: empty key guard ───────────────────────────────────────────────

    #[tokio::test]
    async fn search_returns_empty_when_key_is_empty() {
        let client = reqwest::Client::new();
        let results = search(&client, "", "nature", 5).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn search_parses_response_correctly() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;

        let body = serde_json::json!({
            "hits": [
                {
                    "id": 12345,
                    "duration": 15.5,
                    "userImageURL": "thumb_url",
                    "videos": {
                        "large": { "url": "link_large", "width": 1920, "height": 1080 }
                    }
                }
            ]
        });

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let url = server.uri();
        let results = do_search(&client, &url, "apikey", "nature", 1).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "pixabay_12345");
        assert_eq!(results[0].title, "Nature — Pixabay #12345");
        assert_eq!(results[0].source, "pixabay");
        assert_eq!(results[0].duration, Some(15.5));
        assert_eq!(results[0].thumbnail.as_deref(), Some("thumb_url"));
        assert_eq!(results[0].download_url.as_deref(), Some("link_large"));
    }
}
