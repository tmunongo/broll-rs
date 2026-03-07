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
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
