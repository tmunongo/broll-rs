use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    Sqlx(sqlx::Error),
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            AppError::Sqlx(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Sqlx(e)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    fn status_of(err: AppError) -> StatusCode {
        let resp = err.into_response();
        resp.status()
    }

    #[test]
    fn not_found_gives_404() {
        assert_eq!(status_of(AppError::NotFound("not here".into())), StatusCode::NOT_FOUND);
    }

    #[test]
    fn bad_request_gives_400() {
        assert_eq!(status_of(AppError::BadRequest("bad".into())), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn internal_gives_500() {
        assert_eq!(status_of(AppError::Internal("oops".into())), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn from_anyhow_error() {
        let err: anyhow::Error = anyhow::anyhow!("something went wrong");
        let app_err = AppError::from(err);
        match app_err {
            AppError::Internal(msg) => assert!(msg.contains("something went wrong")),
            _ => panic!("Expected Internal variant"),
        }
    }

    #[test]
    fn from_sqlx_error() {
        // sqlx::Error::RowNotFound is a convenient variant to construct
        let sqlx_err = sqlx::Error::RowNotFound;
        let app_err = AppError::from(sqlx_err);
        match app_err {
            AppError::Sqlx(_) => {}
            _ => panic!("Expected Sqlx variant"),
        }
    }

    #[test]
    fn sqlx_variant_gives_500() {
        let err = AppError::Sqlx(sqlx::Error::RowNotFound);
        assert_eq!(status_of(err), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn app_error_debug() {
        let s = format!("{:?}", AppError::NotFound("x".into()));
        assert!(s.contains("NotFound"));
    }
}
