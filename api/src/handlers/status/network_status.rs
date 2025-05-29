// Network status module for status handler

use sea_orm::DatabaseConnection;
use serde_json::{json, Value};

use super::bitcoin_node::get_bitcoin_node_info;
use super::charm_stats::get_charm_statistics;
use super::db_queries::{
    calculate_time_since_update, get_last_processed_block, get_last_updated_timestamp,
    get_latest_charm_timestamp, get_latest_confirmed_block, get_recent_processed_blocks,
};

/// Gets the current status for a specific network
pub async fn get_network_status(
    conn: &DatabaseConnection,
    _host: &str,
    _port: &str,
    _username: &str,
    _password: &str,
    network_type: &str,
) -> Value {
    // Map network_type to database network value
    let db_network = match network_type {
        "mainnet" => "mainnet",
        _ => "testnet4", // Default to testnet4 for any other value
    };

    // Get the last processed block
    let last_block = match get_last_processed_block(conn, db_network).await {
        Ok(Some(height)) => height,
        Ok(None) => 0,
        Err(_) => 0,
    };

    // Get the latest confirmed block
    let latest_confirmed_block = get_latest_confirmed_block(conn, db_network).await;

    // Get Bitcoin node information
    // We're passing empty values for host, port, username, password since we now use the config directly
    let bitcoin_node_info = get_bitcoin_node_info("", "", "", "", network_type).await;

    // Get recent processed blocks with charm counts
    let recent_blocks = get_recent_processed_blocks(conn, db_network).await;

    // Get charm statistics
    let charm_stats = get_charm_statistics(conn, db_network).await;
    let total_charms = charm_stats
        .get("total_charms")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    // Get the last updated timestamp - try multiple sources
    let last_updated = match get_last_updated_timestamp(conn, db_network).await {
        Ok(Some(timestamp)) => timestamp.to_rfc3339(),
        Ok(None) => {
            // If no transactions, try to get timestamp from charms
            match get_latest_charm_timestamp(conn, db_network).await {
                Ok(Some(timestamp)) => timestamp.to_rfc3339(),
                Ok(None) => "Never".to_string(),
                Err(_) => "Error".to_string(),
            }
        }
        Err(_) => "Error".to_string(),
    };

    // Get the last indexer loop time (same as last updated for now)
    let last_indexer_loop_time = last_updated.clone();

    // Calculate time since last update for status determination
    let time_since_update_seconds = calculate_time_since_update(conn, db_network).await;

    // Determine status based on available data
    let status = if total_charms > 0 {
        if time_since_update_seconds < 0 {
            "indexed" // We have charms but no recent updates
        } else if time_since_update_seconds == -2 {
            "error"
        } else if time_since_update_seconds < 60 {
            "active"
        } else if time_since_update_seconds < 300 {
            "idle"
        } else {
            "inactive"
        }
    } else if last_block > 0 {
        "processing" // We have blocks but no charms yet
    } else {
        "unknown"
    };

    json!({
        "indexer_status": {
            "status": status,
            "last_processed_block": last_block,
            "latest_confirmed_block": latest_confirmed_block,
            "last_updated_at": last_updated,
            "last_indexer_loop_time": last_indexer_loop_time
        },
        "bitcoin_node": bitcoin_node_info,
        "charm_stats": charm_stats,
        "recent_blocks": recent_blocks
    })
}
