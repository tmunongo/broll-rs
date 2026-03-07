use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{CreateProjectRequest, Project, slugify};
use crate::state::AppState;

pub async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Project>>> {
    let projects: Vec<Project> =
        sqlx::query_as("SELECT * FROM projects ORDER BY created_at DESC")
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(projects))
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateProjectRequest>,
) -> AppResult<Json<Project>> {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Project name cannot be empty".into()));
    }

    let slug = slugify(&name);
    if slug.is_empty() {
        return Err(AppError::BadRequest("Project name produces an empty slug".into()));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO projects (id, name, slug, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&slug)
    .bind(&now)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            AppError::BadRequest(format!("Project '{name}' already exists"))
        } else {
            AppError::Sqlx(e)
        }
    })?;

    let project: Project = sqlx::query_as("SELECT * FROM projects WHERE id=?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(project))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let affected = sqlx::query("DELETE FROM projects WHERE id=?")
        .bind(&id)
        .execute(&state.pool)
        .await?
        .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound(format!("Project id={id}")));
    }

    Ok(Json(serde_json::json!({ "deleted": id })))
}
