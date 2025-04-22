// Health check endpoint handler implementation

use crate::services::health::HealthChecker;
use axum::response::IntoResponse;

/// Handler for GET /health - Returns a simple health check response to verify the API is running
pub async fn health_check() -> impl IntoResponse {
    let health_checker = HealthChecker::new();
    if health_checker.check() {
        "OK"
    } else {
        "Service Unavailable"
    }
}
