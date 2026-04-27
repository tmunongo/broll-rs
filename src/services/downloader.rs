use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::config::Config;
use crate::models::DownloadRequest;

pub async fn run(
    pool: SqlitePool,
    config: Arc<Config>,
    http: reqwest::Client,
    req: DownloadRequest,
) {
    set_status(&pool, &req.id, "downloading").await;

    let dir = resolve_dir(&pool, &config, req.project_id.as_deref()).await;
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        tracing::error!("Cannot create download dir {:?}: {e}", dir);
        set_status(&pool, &req.id, "error").await;
        return;
    }

    let result = match req.source.as_str() {
        "youtube" => download_ytdlp(&req.download_url, &dir, &req.id, config.as_ref()).await,
        "archive" => download_archive(&http, &req.download_url, &dir, &req.id).await,
        _ => download_direct(&http, &req.download_url, &dir, &req.id).await,
    };

    match result {
        Ok(path) => {
            let filepath = path.to_string_lossy().to_string();
            sqlx::query("UPDATE videos SET status='complete', filepath=? WHERE id=?")
                .bind(&filepath)
                .bind(&req.id)
                .execute(&pool)
                .await
                .ok();
            tracing::info!("Download complete: {filepath}");
        }
        Err(e) => {
            let err_msg = format!("Download failed for {}: {e:?}", req.id);
            tracing::error!("{}", err_msg);
            let _ = std::fs::write("download_error.txt", err_msg);
            set_status(&pool, &req.id, "error").await;
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn set_status(pool: &SqlitePool, id: &str, status: &str) {
    sqlx::query("UPDATE videos SET status=? WHERE id=?")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await
        .ok();
}

async fn resolve_dir(pool: &SqlitePool, config: &Config, project_id: Option<&str>) -> PathBuf {
    if let Some(pid) = project_id {
        let row: Option<(String,)> = sqlx::query_as("SELECT slug FROM projects WHERE id=?")
            .bind(pid)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);
        if let Some((slug,)) = row {
            return config.downloads_dir.join(slug);
        }
    }
    config.downloads_dir.clone()
}

// ── yt-dlp ────────────────────────────────────────────────────────────────────

async fn download_ytdlp(url: &str, dir: &Path, id: &str, config: &Config) -> Result<PathBuf> {
    let safe_id = id.replace('/', "_");
    let template = dir
        .join(format!("{safe_id}.%(ext)s"))
        .to_string_lossy()
        .to_string();

    let mut args = vec![
        url,
        "-o",
        &template,
        "--no-playlist",
        "--quiet",
        "--merge-output-format",
        "mp4",
        "-f",
        "bestvideo[height<=1080]+bestaudio/best[height<=1080]/best",
    ];

    if let Some(browser) = &config.youtube_cookies_browser {
        args.push("--cookies-from-browser");
        args.push(browser);
    }

    let output = Command::new("yt-dlp")
        .args(&args)
        .output()
        .await
        .context("yt-dlp not found — install with: pip install yt-dlp")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("yt-dlp error: {stderr}");
    }

    // Find the file that was created
    find_file_with_prefix(dir, &safe_id)
        .await
        .context("yt-dlp finished but output file not found")
}

// ── Direct HTTP ───────────────────────────────────────────────────────────────

async fn download_direct(
    client: &reqwest::Client,
    url: &str,
    dir: &Path,
    id: &str,
) -> Result<PathBuf> {
    let ext = ext_from_url(url);
    let safe_id = id.replace('/', "_");
    let dest = dir.join(format!("{safe_id}.{ext}"));
    stream_to_file(client, url, &dest).await?;
    Ok(dest)
}

// ── Archive.org ───────────────────────────────────────────────────────────────

async fn download_archive(
    client: &reqwest::Client,
    base_url: &str,
    dir: &Path,
    id: &str,
) -> Result<PathBuf> {
    let resolved = resolve_archive_url(client, base_url)
        .await
        .unwrap_or_else(|| base_url.to_string());
    download_direct(client, &resolved, dir, id).await
}

async fn resolve_archive_url(client: &reqwest::Client, base_url: &str) -> Option<String> {
    let identifier = base_url.trim_end_matches('/').split('/').next_back()?;
    let meta_url = format!("https://archive.org/metadata/{identifier}");

    let data: serde_json::Value = client.get(&meta_url).send().await.ok()?.json().await.ok()?;
    let files = data.get("files")?.as_array()?;

    let mp4 = files
        .iter()
        .filter(|f| {
            f.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.ends_with(".mp4"))
                .unwrap_or(false)
        })
        .max_by_key(|f| {
            f.get("size")
                .and_then(|s| s.as_str())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0)
        })?;

    let name = mp4.get("name")?.as_str()?;
    Some(format!("https://archive.org/download/{identifier}/{name}"))
}

// ── I/O ───────────────────────────────────────────────────────────────────────

async fn stream_to_file(client: &reqwest::Client, url: &str, dest: &Path) -> Result<()> {
    let mut resp = client.get(url).send().await?.error_for_status()?;
    let mut file = tokio::fs::File::create(dest).await?;
    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    Ok(())
}

async fn find_file_with_prefix(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let mut rd = tokio::fs::read_dir(dir).await.ok()?;
    while let Ok(Some(entry)) = rd.next_entry().await {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(prefix) {
            return Some(entry.path());
        }
    }
    None
}

fn ext_from_url(url: &str) -> &str {
    let path = url.split('?').next().unwrap_or(url);
    path.rsplit('.')
        .next()
        .filter(|e| e.len() <= 4)
        .unwrap_or("mp4")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // ── ext_from_url ──────────────────────────────────────────────────────────

    #[test]
    fn ext_from_simple_url() {
        assert_eq!(ext_from_url("https://example.com/video.mp4"), "mp4");
    }

    #[test]
    fn ext_from_url_with_query_string() {
        assert_eq!(
            ext_from_url("https://example.com/video.webm?token=abc"),
            "webm"
        );
    }

    #[test]
    fn ext_from_url_no_extension() {
        assert_eq!(ext_from_url("https://example.com/video"), "mp4");
    }

    #[test]
    fn ext_from_url_long_extension_falls_back() {
        // ".download" is 8 chars, longer than 4 → fallback to "mp4"
        assert_eq!(ext_from_url("https://example.com/video.download"), "mp4");
    }

    #[test]
    fn ext_from_url_mov() {
        assert_eq!(ext_from_url("https://cdn.example.com/clip.mov"), "mov");
    }

    // ── find_file_with_prefix ─────────────────────────────────────────────────

    #[tokio::test]
    async fn find_file_with_prefix_finds_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("abc123.mp4");
        tokio::fs::File::create(&file_path).await.unwrap();

        let found = find_file_with_prefix(dir.path(), "abc123").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().file_name().unwrap(), "abc123.mp4");
    }

    #[tokio::test]
    async fn find_file_with_prefix_returns_none_when_no_match() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("xyz999.mp4");
        tokio::fs::File::create(&file_path).await.unwrap();

        let found = find_file_with_prefix(dir.path(), "abc123").await;
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn find_file_with_prefix_empty_dir() {
        let dir = tempdir().unwrap();
        let found = find_file_with_prefix(dir.path(), "abc").await;
        assert!(found.is_none());
    }

    // ── download_direct ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn download_direct_streams_to_file() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"fake video bytes"))
            .mount(&server)
            .await;

        let dir = tempdir().unwrap();
        let client = reqwest::Client::new();
        let url = format!("{}/video.mp4", server.uri());

        let path = download_direct(&client, &url, dir.path(), "test_id")
            .await
            .unwrap();
        assert!(path.exists());

        let contents = tokio::fs::read(&path).await.unwrap();
        assert_eq!(contents, b"fake video bytes");
    }

    #[tokio::test]
    async fn download_direct_fails_on_404() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let dir = tempdir().unwrap();
        let client = reqwest::Client::new();
        let url = format!("{}/missing.mp4", server.uri());

        let result = download_direct(&client, &url, dir.path(), "test_id").await;
        assert!(result.is_err());
    }

    // ── resolve_dir (no project_id) ────────────────────────────────────────────

    #[tokio::test]
    async fn resolve_dir_with_no_project_returns_base() {
        let pool = crate::db::init_pool("sqlite::memory:").await.unwrap();
        let config = Config {
            pexels_api_key: "".into(),
            pixabay_api_key: "".into(),
            downloads_dir: std::path::PathBuf::from("/tmp/broll-test"),
            database_url: "sqlite::memory:".into(),
            port: 8000,
            youtube_cookies_browser: None,
        };

        let dir = resolve_dir(&pool, &config, None).await;
        assert_eq!(dir, std::path::PathBuf::from("/tmp/broll-test"));
    }

    #[tokio::test]
    async fn resolve_dir_with_unknown_project_returns_base() {
        let pool = crate::db::init_pool("sqlite::memory:").await.unwrap();
        let config = Config {
            pexels_api_key: "".into(),
            pixabay_api_key: "".into(),
            downloads_dir: std::path::PathBuf::from("/tmp/broll-test"),
            database_url: "sqlite::memory:".into(),
            port: 8000,
            youtube_cookies_browser: None,
        };

        let dir = resolve_dir(&pool, &config, Some("unknown-id")).await;
        assert_eq!(dir, std::path::PathBuf::from("/tmp/broll-test"));
    }

    #[tokio::test]
    async fn resolve_dir_with_known_project_returns_slug_dir() {
        let pool = crate::db::init_pool("sqlite::memory:").await.unwrap();
        sqlx::query(
            "INSERT INTO projects (id, name, slug, created_at) VALUES ('p1','Nature Docs','nature-docs','2024-01-01')"
        )
        .execute(&pool)
        .await
        .unwrap();

        let config = Config {
            pexels_api_key: "".into(),
            pixabay_api_key: "".into(),
            downloads_dir: std::path::PathBuf::from("/tmp/broll-test"),
            database_url: "sqlite::memory:".into(),
            port: 8000,
            youtube_cookies_browser: None,
        };

        let dir = resolve_dir(&pool, &config, Some("p1")).await;
        assert_eq!(dir, std::path::PathBuf::from("/tmp/broll-test/nature-docs"));
    }
}
