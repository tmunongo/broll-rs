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
    match do_search(client, "https://api.pexels.com/videos/search", api_key, query, per_page).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Pexels search failed: {e}");
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
    let resp: PexelsResponse = client
        .get(search_url)
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

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn search_returns_empty_when_key_is_empty() {
        let client = reqwest::Client::new();
        let results = search(&client, "", "nature", 5).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn search_returns_empty_on_http_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/videos/search"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        // Since it hits the real or mock URL, we test failure easily.
        let results = do_search(&client, &server.uri(), "valid_key", "test", 1).await;
        assert!(results.is_err());
    }

    #[tokio::test]
    async fn search_parses_response_correctly() {
        let server = MockServer::start().await;

        let body = serde_json::json!({
            "videos": [
                {
                    "id": 12345,
                    "duration": 15.5,
                    "user": { "name": "John Doe" },
                    "video_files": [
                        { "link": "link_small", "width": 800, "height": 600 },
                        { "link": "link_large", "width": 1920, "height": 1080 }
                    ],
                    "video_pictures": [
                        { "picture": "thumb_url" }
                    ]
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
        assert_eq!(results[0].id, "pexels_12345");
        assert_eq!(results[0].title, "John Doe — nature"); // Updated title format
        assert_eq!(results[0].source, "pexels");
        assert_eq!(results[0].duration, Some(15.5));
        assert_eq!(results[0].thumbnail.as_deref(), Some("thumb_url"));
        assert_eq!(results[0].download_url.as_deref(), Some("link_large"));
    }

    #[test]
    fn pexels_file_fallback_no_files() {
        // If video_files is empty, the sort + find returns None, so we use the fallback
        let mut files: Vec<PexelsFile> = vec![];
        files.sort_by_key(|f| std::cmp::Reverse(f.width.unwrap_or(0)));
        let best = files
            .into_iter()
            .find(|f| f.width.unwrap_or(0) <= 1920)
            .unwrap_or_else(|| PexelsFile {
                link: String::new(),
                width: None,
                height: None,
            });
        assert_eq!(best.link, "");
        assert!(best.width.is_none());
    }

    #[test]
    fn pexels_file_best_picks_largest_under_1920() {
        let mut files = vec![
            PexelsFile { link: "4k.mp4".into(), width: Some(3840), height: Some(2160) },
            PexelsFile { link: "1080p.mp4".into(), width: Some(1920), height: Some(1080) },
            PexelsFile { link: "720p.mp4".into(), width: Some(1280), height: Some(720) },
        ];
        files.sort_by_key(|f| std::cmp::Reverse(f.width.unwrap_or(0)));
        let best = files
            .into_iter()
            .find(|f| f.width.unwrap_or(0) <= 1920)
            .unwrap();
        assert_eq!(best.link, "1080p.mp4");
    }
}
