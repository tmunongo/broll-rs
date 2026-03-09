use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use tokio_util::io::ReaderStream;

use crate::error::{AppError, AppResult};
use crate::models::{LibraryParams, LibraryVideo, TagUpdate};
use crate::state::AppState;

pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<LibraryParams>,
) -> AppResult<Json<Vec<LibraryVideo>>> {
    let videos: Vec<LibraryVideo> = if let Some(pid) = params.project_id {
        sqlx::query_as(
            "SELECT v.*, p.name as project_name
             FROM videos v
             LEFT JOIN projects p ON v.project_id = p.id
             WHERE v.project_id = ?
             ORDER BY v.created_at DESC",
        )
        .bind(&pid)
        .fetch_all(&state.pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT v.*, p.name as project_name
             FROM videos v
             LEFT JOIN projects p ON v.project_id = p.id
             ORDER BY v.created_at DESC",
        )
        .fetch_all(&state.pool)
        .await?
    };

    Ok(Json(videos))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT filepath FROM videos WHERE id=?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?;

    let Some((filepath,)) = row else {
        return Err(AppError::NotFound(format!("id={id}")));
    };

    // Remove file from disk
    if let Some(fp) = filepath {
        let p = std::path::Path::new(&fp);
        if p.exists() {
            tokio::fs::remove_file(p).await.ok();
        }
    }

    sqlx::query("DELETE FROM videos WHERE id=?")
        .bind(&id)
        .execute(&state.pool)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": id })))
}

pub async fn update_tags(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<TagUpdate>,
) -> AppResult<Json<serde_json::Value>> {
    let affected = sqlx::query("UPDATE videos SET tags=? WHERE id=?")
        .bind(&body.tags)
        .bind(&id)
        .execute(&state.pool)
        .await?
        .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound(format!("id={id}")));
    }

    Ok(Json(serde_json::json!({ "id": id, "tags": body.tags })))
}

pub async fn serve_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT filepath FROM videos WHERE id=?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?;

    let Some((Some(fp),)) = row else {
        return Err(AppError::NotFound("File not found".into()));
    };

    let path = std::path::Path::new(&fp);
    if !path.exists() {
        return Err(AppError::NotFound("File missing from disk".into()));
    }

    let file = tokio::fs::File::open(path).await?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("video.mp4")
        .to_string();

    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    Ok((
        [
            ("content-type", "video/mp4".to_string()),
            (
                "content-disposition",
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        body,
    ))
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}
