use axum::{
    extract::{Query, State},
    Json,
};
use futures::future::join_all;

use crate::error::AppResult;
use crate::models::{SearchParams, VideoResult};
use crate::services::{archive, pexels, pixabay, youtube};
use crate::state::AppState;

pub async fn handler(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> AppResult<Json<Vec<VideoResult>>> {
    let q = params.q.trim().to_string();
    let sources = params
        .sources
        .unwrap_or_else(|| "pexels,pixabay,archive,youtube".to_string());

    type Fut = std::pin::Pin<Box<dyn std::future::Future<Output = Vec<VideoResult>> + Send>>;
    let mut futs: Vec<Fut> = vec![];

    if sources.contains("pexels") {
        let client = state.http.clone();
        let key = state.config.pexels_api_key.clone();
        let q2 = q.clone();
        futs.push(Box::pin(async move {
            pexels::search(&client, &key, &q2, 8).await
        }));
    }
    if sources.contains("pixabay") {
        let client = state.http.clone();
        let key = state.config.pixabay_api_key.clone();
        let q2 = q.clone();
        futs.push(Box::pin(async move {
            pixabay::search(&client, &key, &q2, 8).await
        }));
    }
    if sources.contains("archive") {
        let client = state.http.clone();
        let q2 = q.clone();
        futs.push(Box::pin(
            async move { archive::search(&client, &q2, 6).await },
        ));
    }
    if sources.contains("youtube") {
        let q2 = q.clone();
        futs.push(Box::pin(async move { youtube::search(&q2, 8).await }));
    }

    let all = join_all(futs).await;
    let combined: Vec<VideoResult> = all.into_iter().flatten().collect();

    Ok(Json(combined))
}
