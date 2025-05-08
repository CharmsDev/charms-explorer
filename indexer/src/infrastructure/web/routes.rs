// API routes for the indexer

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};

use crate::domain::services::DiagnosticService;

/// Application state shared with all routes
#[derive(Clone)]
pub struct AppState {
    pub diagnostic_service: DiagnosticService,
}

/// Create the API router with all routes
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/api/health", get(health_check))
        .route("/api/diagnostic", get(diagnostic))
        .route("/api/status", get(status))
        .with_state(state)
}

/// Root endpoint
async fn root() -> impl IntoResponse {
    Json(json!({
        "name": "Charms Indexer API",
        "version": "1.0.0",
        "endpoints": [
            "/api/health",
            "/api/diagnostic",
            "/api/status",
        ]
    }))
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Diagnostic endpoint
async fn diagnostic(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    match state.diagnostic_service.diagnose().await {
        value => Ok(Json(value)),
    }
}

/// Status endpoint - returns just the indexer status part of the diagnostic
async fn status(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    match state.diagnostic_service.diagnose().await {
        value => {
            // Extract just the indexer_status part
            if let Some(indexer_status) = value.get("indexer_status") {
                Ok(Json(indexer_status.clone()))
            } else {
                Ok(Json(json!({
                    "error": "Indexer status not available"
                })))
            }
        }
    }
}

/// API error type
pub enum ApiError {
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::InternalError(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}
