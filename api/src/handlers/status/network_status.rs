// Simplified network status module that uses the Summary table

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::{Value, json};

use crate::entity::prelude::*;
use crate::entity::summary;

/// Gets the current status for a specific network using Summary table
pub async fn get_network_status(conn: &DatabaseConnection, network_type: &str) -> Value {
    // Map network_type to database network value
    let db_network = match network_type {
        "mainnet" => "mainnet",
        _ => "testnet4", // Default to testnet4 for any other value
    };

    // Get summary data from the Summary table
    let summary_result = Summary::find()
        .filter(summary::Column::Network.eq(db_network))
        .one(conn)
        .await;

    match summary_result {
        Ok(Some(summary)) => {
            // Build asset type breakdown
            let asset_types = json!([
                {
                    "asset_type": "nft",
                    "count": summary.nft_count
                },
                {
                    "asset_type": "token",
                    "count": summary.token_count
                },
                {
                    "asset_type": "dapp",
                    "count": summary.dapp_count
                },
                {
                    "asset_type": "other",
                    "count": summary.other_count
                }
            ]);

            // Construct the final response
            json!({
                "indexer_status": {
                    "status": determine_status(&summary.last_updated),
                    "last_processed_block": summary.last_processed_block,
                    "latest_confirmed_block": summary.latest_confirmed_block,
                    "last_updated_at": summary.last_updated.to_string(),
                    "last_indexer_loop_time": summary.last_updated.to_string()
                },
                "bitcoin_node": {
                    "status": summary.bitcoin_node_status,
                    "network": network_type,
                    "block_count": summary.bitcoin_node_block_count,
                    "best_block_hash": summary.bitcoin_node_best_block_hash
                },
                "charm_stats": {
                    "total_charms": summary.total_charms,
                    "total_transactions": summary.total_transactions,
                    "confirmed_transactions": summary.confirmed_transactions,
                    "confirmation_rate": summary.confirmation_rate,
                    "charms_by_asset_type": asset_types
                },
                "tag_stats": {
                    "charms_cast_count": summary.charms_cast_count,
                    "bro_count": summary.bro_count,
                    "dex_orders_count": summary.dex_orders_count
                }
            })
        }
        _ => {
            // Fallback to default values if query fails
            json!({
                "indexer_status": {
                    "status": "unknown",
                    "last_processed_block": 0,
                    "latest_confirmed_block": 0,
                    "last_updated_at": "Never",
                    "last_indexer_loop_time": "Never"
                },
                "bitcoin_node": {
                    "status": "unknown",
                    "network": network_type,
                    "block_count": 0,
                    "best_block_hash": "unknown"
                },
                "charm_stats": {
                    "total_charms": 0,
                    "total_transactions": 0,
                    "confirmed_transactions": 0,
                    "confirmation_rate": 0,
                    "charms_by_asset_type": []
                },
                "tag_stats": {
                    "charms_cast_count": 0,
                    "bro_count": 0,
                    "dex_orders_count": 0
                }
            })
        }
    }
}

/// Helper function to determine status based on last_updated timestamp
fn determine_status(last_updated: &chrono::DateTime<chrono::Utc>) -> &'static str {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(*last_updated);
    let seconds = duration.num_seconds();

    if seconds < 60 {
        "active"
    } else if seconds < 300 {
        "idle"
    } else {
        "inactive"
    }
}
