mod config;
mod db;
mod error;
mod models;
mod routes;
mod services;
mod state;

#[cfg(test)]
mod tests;

use axum::{
    response::Html,
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use state::AppState;

const INDEX_HTML: &str = include_str!("../templates/index.html");

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

pub fn build_app(state: AppState) -> Router {
    Router::new()
        // UI
        .route("/", get(index))
        .nest_service(
            "/app.css",
            tower_http::services::ServeFile::new("templates/app.css"),
        )
        .nest_service(
            "/app.js",
            tower_http::services::ServeFile::new("templates/app.js"),
        )
        // Search
        .route("/api/search", get(routes::search::handler))
        // Download
        .route("/api/download", post(routes::download::start))
        .route("/api/download/status/:id", get(routes::download::status))
        .route("/api/download/retry/:id", post(routes::download::retry))
        // Library
        .route("/api/library", get(routes::library::list))
        .route("/api/library/:id", delete(routes::library::delete))
        .route("/api/library/:id/tags", patch(routes::library::update_tags))
        .route("/api/library/file/:id", get(routes::library::serve_file))
        // Projects
        .route(
            "/api/projects",
            get(routes::projects::list).post(routes::projects::create),
        )
        .route("/api/projects/:id", delete(routes::projects::delete))
        .layer(CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "broll_rs=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    tracing::info!("Downloads dir: {:?}", config.downloads_dir);

    tokio::fs::create_dir_all(&config.downloads_dir).await?;

    let pool = db::init_pool(&config.database_url).await?;
    tracing::info!("Database ready: {}", config.database_url);

    let http = reqwest::Client::builder()
        .user_agent("broll-harness/1.0")
        .timeout(std::time::Duration::from_secs(600)) // 10 minutes to allow slow video downloads
        .build()?;

    let state = AppState {
        pool,
        config: Arc::new(config.clone()),
        http,
    };

    let app = build_app(state);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
