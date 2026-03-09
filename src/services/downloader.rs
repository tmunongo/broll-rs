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
        "youtube" => download_ytdlp(&req.download_url, &dir, &req.id).await,
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
            tracing::error!("Download failed for {}: {e}", req.id);
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

async fn download_ytdlp(url: &str, dir: &Path, id: &str) -> Result<PathBuf> {
    let safe_id = id.replace('/', "_");
    let template = dir
        .join(format!("{safe_id}.%(ext)s"))
        .to_string_lossy()
        .to_string();

    let output = Command::new("yt-dlp")
        .args([
            url,
            "-o",
            &template,
            "--no-playlist",
            "--quiet",
            "--merge-output-format",
            "mp4",
            "-f",
            "bestvideo[height<=1080]+bestaudio/best[height<=1080]/best",
        ])
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
