use serde::{Deserialize, Serialize};

// ── Search result returned to UI ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoResult {
    pub id: String,
    pub title: String,
    pub source: String,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub preview_url: Option<String>,
    pub download_url: Option<String>,
    pub license: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

// ── Download request from UI ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DownloadRequest {
    pub id: String,
    pub title: String,
    pub source: String,
    pub download_url: String,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub project_id: Option<String>,
}

// ── Library video (DB row + joined project name) ──────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct LibraryVideo {
    pub id: String,
    pub title: String,
    pub source: String,
    pub filepath: Option<String>,
    pub duration: Option<f64>,
    pub tags: Option<String>,
    pub thumbnail: Option<String>,
    pub original_url: Option<String>,
    pub status: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub created_at: String,
}

// ── Project ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
}

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub sources: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LibraryParams {
    pub project_id: Option<String>,
}

// ── Tag update ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TagUpdate {
    pub tags: String,
}

// ── Download status response ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub id: String,
    pub status: String,
    pub filepath: Option<String>,
}

/// Slugify a project name to a filesystem-safe directory name.
pub fn slugify(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── slugify ────────────────────────────────────────────────────────────────

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn slugify_special_chars() {
        assert_eq!(slugify("B-Roll: Nature & Wildlife!"), "b-roll-nature-wildlife");
    }

    #[test]
    fn slugify_leading_trailing_whitespace() {
        assert_eq!(slugify("  my project  "), "my-project");
    }

    #[test]
    fn slugify_multiple_spaces() {
        assert_eq!(slugify("a   b"), "a-b");
    }

    #[test]
    fn slugify_already_slug() {
        assert_eq!(slugify("already-slug"), "already-slug");
    }

    #[test]
    fn slugify_empty() {
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn slugify_only_special_chars() {
        assert_eq!(slugify("!!!"), "");
    }

    #[test]
    fn slugify_unicode() {
        // Non-ASCII alphanumeric chars are treated as alphanumeric by is_alphanumeric()
        // e.g. é is alphanumeric in Rust
        let result = slugify("café");
        assert!(!result.is_empty());
    }

    // ── VideoResult serde ──────────────────────────────────────────────────────

    #[test]
    fn video_result_serializes() {
        let v = VideoResult {
            id: "test_1".into(),
            title: "Test Video".into(),
            source: "pexels".into(),
            duration: Some(30.5),
            thumbnail: Some("https://example.com/thumb.jpg".into()),
            preview_url: Some("https://example.com/preview.mp4".into()),
            download_url: Some("https://example.com/video.mp4".into()),
            license: Some("CC0".into()),
            width: Some(1920),
            height: Some(1080),
        };
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("\"id\":\"test_1\""));
        assert!(json.contains("\"source\":\"pexels\""));
        assert!(json.contains("\"duration\":30.5"));
    }

    #[test]
    fn video_result_deserializes() {
        let json = r#"{
            "id": "archive_foo",
            "title": "Foo",
            "source": "archive",
            "duration": null,
            "thumbnail": null,
            "preview_url": null,
            "download_url": null,
            "license": null,
            "width": null,
            "height": null
        }"#;
        let v: VideoResult = serde_json::from_str(json).unwrap();
        assert_eq!(v.id, "archive_foo");
        assert_eq!(v.source, "archive");
        assert!(v.duration.is_none());
    }

    #[test]
    fn download_request_deserializes() {
        let json = r#"{
            "id":"vid1","title":"A","source":"pexels",
            "download_url":"http://x.com/a.mp4","thumbnail":null,
            "duration":null,"project_id":null
        }"#;
        let req: DownloadRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.id, "vid1");
        assert_eq!(req.source, "pexels");
        assert!(req.project_id.is_none());
    }

    #[test]
    fn library_video_derives_clone() {
        let v = LibraryVideo {
            id: "1".into(),
            title: "T".into(),
            source: "s".into(),
            filepath: None,
            duration: None,
            tags: None,
            thumbnail: None,
            original_url: None,
            status: "pending".into(),
            project_id: None,
            project_name: None,
            created_at: "2024-01-01T00:00:00Z".into(),
        };
        let cloned = v.clone();
        assert_eq!(cloned.id, "1");
    }

    #[test]
    fn project_derives_clone() {
        let p = Project {
            id: "p1".into(),
            name: "My Project".into(),
            slug: "my-project".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
        };
        let cloned = p.clone();
        assert_eq!(cloned.slug, "my-project");
    }

    #[test]
    fn tag_update_deserializes() {
        let json = r#"{"tags":"nature,wildlife"}"#;
        let t: TagUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(t.tags, "nature,wildlife");
    }

    #[test]
    fn search_params_deserializes() {
        let json = r#"{"q":"nature","sources":"pexels,pixabay"}"#;
        let p: SearchParams = serde_json::from_str(json).unwrap();
        assert_eq!(p.q, "nature");
        assert_eq!(p.sources.unwrap(), "pexels,pixabay");
    }

    #[test]
    fn library_params_optional_project() {
        let json = r#"{}"#;
        let p: LibraryParams = serde_json::from_str(json).unwrap();
        assert!(p.project_id.is_none());
    }

    #[test]
    fn status_response_serializes() {
        let s = StatusResponse {
            id: "id1".into(),
            status: "complete".into(),
            filepath: Some("/downloads/id1.mp4".into()),
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"status\":\"complete\""));
    }
}
