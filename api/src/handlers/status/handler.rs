// Main status handler implementation

use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use super::network_status::get_network_status;
use crate::handlers::AppState;

/// Handler for GET /status - Returns the indexer status
pub async fn get_indexer_status(State(app_state): State<AppState>) -> impl IntoResponse {
    let conn = app_state.repositories.charm.get_connection();
    let config = &app_state.config;

    // Get status for testnet4
    let testnet4_status = get_network_status(
        conn,
        &config.bitcoin_testnet4_rpc_host,
        &config.bitcoin_testnet4_rpc_port,
        &config.bitcoin_testnet4_rpc_username,
        &config.bitcoin_testnet4_rpc_password,
        "testnet4",
    )
    .await;

    // Get status for mainnet
    let mainnet_status = get_network_status(
        conn,
        &config.bitcoin_mainnet_rpc_host,
        &config.bitcoin_mainnet_rpc_port,
        &config.bitcoin_mainnet_rpc_username,
        &config.bitcoin_mainnet_rpc_password,
        "mainnet",
    )
    .await;

    // Return the combined status with network separation
    let combined_status = json!({
        "networks": {
            "testnet4": testnet4_status,
            "mainnet": mainnet_status
        }
    });

    Json(combined_status)
}
