// Database diagnostic endpoint handler implementation

use axum::{extract::State, response::IntoResponse, Json};
use std::sync::Arc;

use crate::db::repositories::Repositories;
use crate::services::diagnostic::DiagnosticService;

/// Handler for GET /diagnose - Returns detailed database diagnostic information
pub async fn diagnose_database(State(repositories): State<Arc<Repositories>>) -> impl IntoResponse {
    // Create diagnostic service with a reference to the database connection
    let diagnostic_service = DiagnosticService::new(repositories.charm.get_connection());

    // Run diagnostic checks
    let diagnostic_result = diagnostic_service.diagnose().await;

    // Return JSON response
    Json(diagnostic_result)
}
