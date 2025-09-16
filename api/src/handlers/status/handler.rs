// Simplified status handler implementation

use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use super::network_status::get_network_status;
use crate::handlers::AppState;

/// Handler for GET /status - Returns the indexer status
pub async fn get_indexer_status(State(app_state): State<AppState>) -> impl IntoResponse {
    let conn = app_state.repositories.charm.get_connection();

    // Run both network status queries in parallel
    let (testnet4_status, mainnet_status) = tokio::join!(
        get_network_status(conn, "testnet4"),
        get_network_status(conn, "mainnet")
    );

    // Return the combined status with network separation
    let combined_status = json!({
        "networks": {
            "testnet4": testnet4_status,
            "mainnet": mainnet_status
        }
    });

    Json(combined_status)
}
