// Database diagnostic endpoint handler implementation

use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;
use std::collections::HashMap;

use crate::handlers::AppState;
use crate::services::diagnostic::DiagnosticService;

/// Handler for GET /diagnose - Returns detailed database diagnostic information
pub async fn diagnose_database(State(app_state): State<AppState>) -> impl IntoResponse {
    // Create diagnostic service with a reference to the database connection and config
    let diagnostic_service = DiagnosticService::new(
        app_state.repositories.charm.get_connection(),
        &app_state.config,
    );

    // Run diagnostic checks
    let diagnostic_result = diagnostic_service.diagnose().await;

    // Format the response to match the expected structure
    let mut response = HashMap::new();

    // Add version number to identify the diagnostic format
    response.insert("version", json!("1.1.0"));

    // Add Bitcoin RPC test information
    if let Some(bitcoin_rpc) = diagnostic_result.get("bitcoin_rpc") {
        response.insert("bitcoin_rpc_test", bitcoin_rpc.clone());
    }

    // Add table counts information
    let table_counts = if let Some(tables) = diagnostic_result.get("tables") {
        if let Some(tables_array) = tables.get("tables").and_then(|t| t.as_array()) {
            // Convert the tables array to a map of table name -> count
            let mut counts = HashMap::new();
            for table in tables_array {
                if let (Some(name), Some(count)) = (
                    table.get("name").and_then(|n| n.as_str()),
                    table.get("row_count").and_then(|c| c.as_i64()),
                ) {
                    counts.insert(name.to_string(), count);
                }
            }
            json!(counts)
        } else {
            json!({})
        }
    } else {
        json!({})
    };
    response.insert("table_counts", table_counts);

    // Add database connection information if available
    if let Some(db_connection) = diagnostic_result.get("db_connection") {
        response.insert("db_connection", db_connection.clone());
    }

    // Return JSON response
    Json(json!(response))
}
