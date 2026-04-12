// Integration tests for all route handlers using in-memory SQLite.
//
// We build the full Axum router (same as main) and use tower's `oneshot` helper
// to dispatch HTTP requests directly without binding a real TCP socket.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{delete, get, patch, post},
    Router,
};
use serde_json::Value;
use tower::ServiceExt; // for `oneshot`

use crate::{
    config::Config,
    db,
    routes,
    state::AppState,
};

// ── Shared helpers ────────────────────────────────────────────────────────────

async fn build_app() -> Router {
    let pool = db::init_pool("sqlite::memory:")
        .await
        .expect("in-memory db init failed");

    let config = Arc::new(Config {
        pexels_api_key: String::new(),
        pixabay_api_key: String::new(),
        downloads_dir: std::path::PathBuf::from("/tmp/broll-test-routes"),
        database_url: "sqlite::memory:".into(),
        port: 8000,
    });

    let http = reqwest::Client::new();
    let state = AppState { pool, config, http };

    crate::build_app(state)
}

#[tokio::test]
async fn index_returns_html() {
    let app = build_app().await;
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap().to_str().unwrap(),
        "text/html; charset=utf-8"
    );
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ── Projects routes ───────────────────────────────────────────────────────────

#[tokio::test]
async fn list_projects_empty() {
    let app = build_app().await;
    let req = Request::builder()
        .uri("/api/projects")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json, Value::Array(vec![]));
}

#[tokio::test]
async fn create_project_success() {
    let app = build_app().await;
    let body = serde_json::json!({ "name": "Nature Docs" });
    let req = Request::builder()
        .method("POST")
        .uri("/api/projects")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["name"], "Nature Docs");
    assert_eq!(json["slug"], "nature-docs");
}

#[tokio::test]
async fn create_project_empty_name_returns_400() {
    let app = build_app().await;
    let body = serde_json::json!({ "name": "   " });
    let req = Request::builder()
        .method("POST")
        .uri("/api/projects")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_project_special_chars_only_returns_400() {
    let app = build_app().await;
    let body = serde_json::json!({ "name": "!!!" });
    let req = Request::builder()
        .method("POST")
        .uri("/api/projects")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn delete_project_not_found() {
    let app = build_app().await;
    let req = Request::builder()
        .method("DELETE")
        .uri("/api/projects/nonexistent-id")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_and_delete_project() {
    let app = build_app().await;

    // Create
    let body = serde_json::json!({ "name": "Temp Project" });
    let req = Request::builder()
        .method("POST")
        .uri("/api/projects")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let id = json["id"].as_str().unwrap().to_string();

    // Delete
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/projects/{id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["deleted"], id.as_str());
}

#[tokio::test]
async fn create_duplicate_project_returns_400() {
    let app = build_app().await;

    let body = serde_json::json!({ "name": "Unique Project" });

    let req1 = Request::builder()
        .method("POST")
        .uri("/api/projects")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);

    let req2 = Request::builder()
        .method("POST")
        .uri("/api/projects")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp2 = app.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::BAD_REQUEST);
}

// ── Library routes ────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_library_empty() {
    let app = build_app().await;
    let req = Request::builder()
        .uri("/api/library")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json, Value::Array(vec![]));
}

#[tokio::test]
async fn delete_library_item_not_found() {
    let app = build_app().await;
    let req = Request::builder()
        .method("DELETE")
        .uri("/api/library/nonexistent-id")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_tags_not_found() {
    let app = build_app().await;
    let body = serde_json::json!({ "tags": "nature" });
    let req = Request::builder()
        .method("PATCH")
        .uri("/api/library/nonexistent-id/tags")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn serve_file_not_found() {
    let app = build_app().await;
    let req = Request::builder()
        .uri("/api/library/nonexistent-id/file")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Download routes ───────────────────────────────────────────────────────────

#[tokio::test]
async fn download_status_not_found() {
    let app = build_app().await;
    let req = Request::builder()
        .uri("/api/download/status/nonexistent-id")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn start_download_creates_record() {
    let app = build_app().await;
    let body = serde_json::json!({
        "id": "vid-abc-123",
        "title": "Sample Video",
        "source": "pexels",
        "download_url": "https://example.com/video.mp4",
        "thumbnail": null,
        "duration": 30.0,
        "project_id": null
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/download")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["id"], "vid-abc-123");
    assert_eq!(json["status"], "pending");
}

#[tokio::test]
async fn download_status_after_start() {
    let app = build_app().await;

    // Start download
    let body = serde_json::json!({
        "id": "vid-status-test",
        "title": "Status Test Video",
        "source": "archive",
        "download_url": "https://example.com/video.mp4",
        "thumbnail": null,
        "duration": null,
        "project_id": null
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/download")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let post_resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(post_resp.status(), StatusCode::OK);

    // Check status
    let req = Request::builder()
        .uri("/api/download/status/vid-status-test")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["id"], "vid-status-test");
    assert!(json["status"].as_str().is_some());
}

#[tokio::test]
async fn start_download_existing_complete_returns_existing() {
    // If a record already has status='complete', starting again just returns it
    let pool = db::init_pool("sqlite::memory:").await.unwrap();

    // Pre-insert a completed record
    sqlx::query(
        "INSERT INTO videos (id, title, source, status, created_at) VALUES ('done-vid', 'Done', 'pexels', 'complete', '2024-01-01')"
    )
    .execute(&pool)
    .await
    .unwrap();

    let config = Arc::new(Config {
        pexels_api_key: String::new(),
        pixabay_api_key: String::new(),
        downloads_dir: std::path::PathBuf::from("/tmp/broll-test-routes"),
        database_url: "sqlite::memory:".into(),
        port: 8000,
    });

    let state = AppState { pool, config, http: reqwest::Client::new() };
    let app = Router::new()
        .route("/api/download", post(routes::download::start))
        .with_state(state);

    let body = serde_json::json!({
        "id": "done-vid",
        "title": "Done",
        "source": "pexels",
        "download_url": "https://example.com/done.mp4",
        "thumbnail": null,
        "duration": null,
        "project_id": null
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/download")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "complete");
}

// ── Search route ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn search_with_no_real_keys_returns_empty_or_results() {
    let app = build_app().await;
    // With empty API keys, pexels + pixabay return empty. Archive + youtube may
    // fail in CI (no network). Either way, the handler should return 200 with an array.
    let req = Request::builder()
        .uri("/api/search?q=nature&sources=pexels,pixabay")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json.is_array());
}

// ── Library with seed data ────────────────────────────────────────────────────

#[tokio::test]
async fn list_library_with_seed_data() {
    let pool = db::init_pool("sqlite::memory:").await.unwrap();

    sqlx::query(
        "INSERT INTO videos (id, title, source, status, created_at) VALUES ('v1','A','pexels','complete','2024-01-01')"
    )
    .execute(&pool)
    .await
    .unwrap();

    let config = Arc::new(Config {
        pexels_api_key: String::new(),
        pixabay_api_key: String::new(),
        downloads_dir: "/tmp/broll".into(),
        database_url: "sqlite::memory:".into(),
        port: 8000,
    });

    let state = AppState { pool, config, http: reqwest::Client::new() };
    let app = Router::new()
        .route("/api/library", get(routes::library::list))
        .with_state(state);

    let req = Request::builder()
        .uri("/api/library")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "v1");
}

#[tokio::test]
async fn update_tags_success() {
    let pool = db::init_pool("sqlite::memory:").await.unwrap();

    sqlx::query(
        "INSERT INTO videos (id, title, source, status, created_at) VALUES ('v2','B','archive','pending','2024-01-01')"
    )
    .execute(&pool)
    .await
    .unwrap();

    let config = Arc::new(Config {
        pexels_api_key: String::new(),
        pixabay_api_key: String::new(),
        downloads_dir: "/tmp/broll".into(),
        database_url: "sqlite::memory:".into(),
        port: 8000,
    });

    let state = AppState { pool, config, http: reqwest::Client::new() };
    let app = Router::new()
        .route("/api/library/:id/tags", patch(routes::library::update_tags))
        .with_state(state);

    let body = serde_json::json!({ "tags": "nature,wildlife" });
    let req = Request::builder()
        .method("PATCH")
        .uri("/api/library/v2/tags")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["tags"], "nature,wildlife");
}

#[tokio::test]
async fn delete_library_item_success() {
    let pool = db::init_pool("sqlite::memory:").await.unwrap();

    sqlx::query(
        "INSERT INTO videos (id, title, source, status, created_at) VALUES ('v3','C','pixabay','complete','2024-01-01')"
    )
    .execute(&pool)
    .await
    .unwrap();

    let config = Arc::new(Config {
        pexels_api_key: String::new(),
        pixabay_api_key: String::new(),
        downloads_dir: "/tmp/broll".into(),
        database_url: "sqlite::memory:".into(),
        port: 8000,
    });

    let state = AppState { pool, config, http: reqwest::Client::new() };
    let app = Router::new()
        .route("/api/library/:id", delete(routes::library::delete))
        .with_state(state);

    let req = Request::builder()
        .method("DELETE")
        .uri("/api/library/v3")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["deleted"], "v3");
}

#[tokio::test]
async fn list_library_filtered_by_project() {
    let pool = db::init_pool("sqlite::memory:").await.unwrap();

    sqlx::query(
        "INSERT INTO projects (id, name, slug, created_at) VALUES ('proj1','NatDocs','nat-docs','2024-01-01')"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO videos (id, title, source, status, project_id, created_at) VALUES ('v4','D','pexels','complete','proj1','2024-01-02')"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO videos (id, title, source, status, project_id, created_at) VALUES ('v5','E','pexels','pending', null,'2024-01-03')"
    )
    .execute(&pool)
    .await
    .unwrap();

    let config = Arc::new(Config {
        pexels_api_key: String::new(),
        pixabay_api_key: String::new(),
        downloads_dir: "/tmp/broll".into(),
        database_url: "sqlite::memory:".into(),
        port: 8000,
    });

    let state = AppState { pool, config, http: reqwest::Client::new() };
    let app = Router::new()
        .route("/api/library", get(routes::library::list))
        .with_state(state);

    let req = Request::builder()
        .uri("/api/library?project_id=proj1")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "v4");
}

// ── Retry download routes ─────────────────────────────────────────────────────

#[tokio::test]
async fn retry_download_not_found() {
    let app = build_app().await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/download/retry/nonexistent-id")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn retry_download_rejects_non_error_status() {
    let pool = db::init_pool("sqlite::memory:").await.unwrap();

    // Insert a record with status='complete' — retry should be rejected
    sqlx::query(
        "INSERT INTO videos (id, title, source, status, original_url, created_at) \
         VALUES ('ok-vid', 'OK', 'pexels', 'complete', 'https://example.com/v.mp4', '2024-01-01')"
    )
    .execute(&pool)
    .await
    .unwrap();

    let config = Arc::new(Config {
        pexels_api_key: String::new(),
        pixabay_api_key: String::new(),
        downloads_dir: std::path::PathBuf::from("/tmp/broll-test-retry"),
        database_url: "sqlite::memory:".into(),
        port: 8000,
    });

    let state = AppState { pool, config, http: reqwest::Client::new() };
    let app = Router::new()
        .route("/api/download/retry/:id", post(routes::download::retry))
        .with_state(state);

    let req = Request::builder()
        .method("POST")
        .uri("/api/download/retry/ok-vid")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn retry_download_resets_error_to_pending() {
    let pool = db::init_pool("sqlite::memory:").await.unwrap();

    // Insert a failed record with an original_url
    sqlx::query(
        "INSERT INTO videos (id, title, source, status, original_url, created_at) \
         VALUES ('fail-vid', 'Failed', 'pexels', 'error', 'https://example.com/v.mp4', '2024-01-01')"
    )
    .execute(&pool)
    .await
    .unwrap();

    let config = Arc::new(Config {
        pexels_api_key: String::new(),
        pixabay_api_key: String::new(),
        downloads_dir: std::path::PathBuf::from("/tmp/broll-test-retry"),
        database_url: "sqlite::memory:".into(),
        port: 8000,
    });

    let state = AppState { pool, config, http: reqwest::Client::new() };
    let app = Router::new()
        .route("/api/download/retry/:id", post(routes::download::retry))
        .with_state(state);

    let req = Request::builder()
        .method("POST")
        .uri("/api/download/retry/fail-vid")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["id"], "fail-vid");
    assert_eq!(json["status"], "pending");
}

#[tokio::test]
async fn retry_download_no_original_url_returns_400() {
    let pool = db::init_pool("sqlite::memory:").await.unwrap();

    // Insert a failed record WITHOUT an original_url
    sqlx::query(
        "INSERT INTO videos (id, title, source, status, created_at) \
         VALUES ('no-url-vid', 'No URL', 'pexels', 'error', '2024-01-01')"
    )
    .execute(&pool)
    .await
    .unwrap();

    let config = Arc::new(Config {
        pexels_api_key: String::new(),
        pixabay_api_key: String::new(),
        downloads_dir: std::path::PathBuf::from("/tmp/broll-test-retry"),
        database_url: "sqlite::memory:".into(),
        port: 8000,
    });

    let state = AppState { pool, config, http: reqwest::Client::new() };
    let app = Router::new()
        .route("/api/download/retry/:id", post(routes::download::retry))
        .with_state(state);

    let req = Request::builder()
        .method("POST")
        .uri("/api/download/retry/no-url-vid")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
