use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

use crate::db::DbError;

#[derive(Error, Debug)]
pub enum ExplorerError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Invalid request: {0}")]
    #[allow(dead_code)] // Reserved for validation errors
    InvalidRequest(String),
    #[error("Internal error: {0}")]
    #[allow(dead_code)] // Reserved for general errors
    InternalError(String),
}

pub type ExplorerResult<T> = Result<T, ExplorerError>;

impl IntoResponse for ExplorerError {
    fn into_response(self) -> Response {
        let (status, err_msg) = match self {
            ExplorerError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ExplorerError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ExplorerError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ExplorerError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": err_msg
        }));

        (status, body).into_response()
    }
}

// DbError to ExplorerError conversion implementation
impl From<DbError> for ExplorerError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::ConnectionError(msg) => ExplorerError::DatabaseError(msg),
            DbError::QueryError(msg) => {
                if msg.contains("not found") {
                    ExplorerError::NotFound(msg)
                } else {
                    ExplorerError::DatabaseError(msg)
                }
            }
        }
    }
}
