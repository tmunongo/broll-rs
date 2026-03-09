use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;

use crate::error::{AppError, AppResult};
use crate::models::{DownloadRequest, LibraryVideo, StatusResponse};
use crate::services::downloader;
use crate::state::AppState;

pub async fn start(
    State(state): State<AppState>,
    Json(req): Json<DownloadRequest>,
) -> AppResult<Json<LibraryVideo>> {
    // Check for existing record
    let existing: Option<LibraryVideo> = sqlx::query_as(
        "SELECT v.*, p.name as project_name
         FROM videos v
         LEFT JOIN projects p ON v.project_id = p.id
         WHERE v.id = ?",
    )
    .bind(&req.id)
    .fetch_optional(&state.pool)
    .await?;

    if let Some(ref rec) = existing {
        if rec.status == "complete" {
            return Ok(Json(rec.clone()));
        }
    }

    let now = Utc::now().to_rfc3339();

    if existing.is_some() {
        sqlx::query("UPDATE videos SET status='pending', project_id=? WHERE id=?")
            .bind(&req.project_id)
            .bind(&req.id)
            .execute(&state.pool)
            .await?;
    } else {
        sqlx::query(
            "INSERT INTO videos (id, title, source, thumbnail, original_url, duration, status, project_id, created_at)
             VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, ?)",
        )
        .bind(&req.id)
        .bind(&req.title)
        .bind(&req.source)
        .bind(&req.thumbnail)
        .bind(&req.download_url)
        .bind(req.duration)
        .bind(&req.project_id)
        .bind(&now)
        .execute(&state.pool)
        .await?;
    }

    // Fetch the freshly-inserted record
    let record: LibraryVideo = sqlx::query_as(
        "SELECT v.*, p.name as project_name
         FROM videos v
         LEFT JOIN projects p ON v.project_id = p.id
         WHERE v.id = ?",
    )
    .bind(&req.id)
    .fetch_one(&state.pool)
    .await?;

    // Spawn background download — doesn't block the response
    let pool = state.pool.clone();
    let config = state.config.clone();
    let http = state.http.clone();
    tokio::spawn(async move {
        downloader::run(pool, config, http, req).await;
    });

    Ok(Json(record))
}

pub async fn status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<StatusResponse>> {
    let row: Option<(String, String, Option<String>)> =
        sqlx::query_as("SELECT id, status, filepath FROM videos WHERE id=?")
            .bind(&id)
            .fetch_optional(&state.pool)
            .await?;

    match row {
        Some((id, status, filepath)) => Ok(Json(StatusResponse {
            id,
            status,
            filepath,
        })),
        None => Err(AppError::NotFound(format!(
            "No download record for id={id}"
        ))),
    }
}
