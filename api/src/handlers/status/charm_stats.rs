// Charm statistics module for status handler

use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde_json::{json, Value};

use super::db_queries::{
    count_confirmed_valid_transactions, count_table_rows, count_valid_transactions,
};

/// Gets statistics about indexed charms for a specific network
pub async fn get_charm_statistics(conn: &DatabaseConnection, network: &str) -> Value {
    // Get total number of charms
    let total_charms = count_table_rows(conn, "charms", network).await;

    // Get total number of transactions associated with valid charms
    let total_transactions = count_valid_transactions(conn, network).await;

    // Get confirmed transactions count (only for valid charms)
    let confirmed_transactions = count_confirmed_valid_transactions(conn, network).await;

    // Calculate confirmation rate
    let confirmation_rate = if total_transactions > 0 {
        (confirmed_transactions as f64 / total_transactions as f64 * 100.0).round() as i64
    } else {
        0
    };

    // Get charms by asset type
    let charms_by_asset_type = get_charms_by_asset_type(conn, network).await;

    // Get recent charms
    let recent_charms = get_recent_charms(conn, network).await;

    json!({
        "total_charms": total_charms,
        "total_transactions": total_transactions,
        "confirmed_transactions": confirmed_transactions,
        "confirmation_rate": confirmation_rate,
        "charms_by_asset_type": charms_by_asset_type,
        "recent_charms": recent_charms
    })
}

/// Gets charms grouped by asset type using improved backend detection logic
async fn get_charms_by_asset_type(conn: &DatabaseConnection, network: &str) -> Value {
    // First, get all charms with their data, excluding empty spell charms
    let query = format!(
        "
        SELECT txid, charmid, data, asset_type
        FROM charms
        WHERE network = '{}' AND NOT (
            data::jsonb -> 'data' = '{{}}'::jsonb AND 
            data::jsonb ->> 'type' = 'spell' AND 
            (data::jsonb ->> 'detected')::boolean = true
        )
    ",
        network
    );

    match conn
        .query_all(Statement::from_string(conn.get_database_backend(), query))
        .await
    {
        Ok(rows) => {
            // Initialize counters for each type
            let mut nft_count = 0;
            let mut token_count = 0;
            let mut dapp_count = 0;
            let mut other_count = 0;

            // Debug: Log the number of rows
            tracing::info!("Processing {} charms for asset type detection", rows.len());

            // Process each charm and detect its type
            for row in rows.iter() {
                let txid = row.try_get::<String>("", "txid").unwrap_or_default();
                let data_str = row.try_get::<String>("", "data").unwrap_or_default();
                let asset_type = row.try_get::<String>("", "asset_type").unwrap_or_default();

                // Try to parse the data as JSON
                if let Ok(data) = serde_json::from_str::<Value>(&data_str) {
                    // Apply improved detection logic
                    let detected_type = detect_charm_type(&data, &asset_type);

                    // Debug: Log the detected type for each charm
                    tracing::info!("Charm {} detected as type: {}", txid, detected_type);

                    match detected_type.as_str() {
                        "nft" => nft_count += 1,
                        "token" => token_count += 1,
                        "dapp" => dapp_count += 1,
                        _ => other_count += 1,
                    }
                } else {
                    // If we can't parse the data, use the asset_type
                    tracing::warn!(
                        "Failed to parse data for charm {}, using asset_type: {}",
                        txid,
                        asset_type
                    );
                    if asset_type == "nft" {
                        nft_count += 1;
                    } else if asset_type == "token" {
                        token_count += 1;
                    } else if asset_type == "dapp" {
                        dapp_count += 1;
                    } else {
                        other_count += 1;
                    }
                }
            }

            // Debug: Log the final counts
            tracing::info!(
                "Final counts - NFT: {}, Token: {}, dApp: {}, Other: {}",
                nft_count,
                token_count,
                dapp_count,
                other_count
            );

            // Create the result array with the counts
            let mut result = Vec::new();

            if nft_count > 0 {
                result.push(json!({
                    "asset_type": "nft",
                    "count": nft_count,
                }));
            }

            if token_count > 0 {
                result.push(json!({
                    "asset_type": "token",
                    "count": token_count,
                }));
            }

            if dapp_count > 0 {
                result.push(json!({
                    "asset_type": "dapp",
                    "count": dapp_count,
                }));
            }

            if other_count > 0 {
                result.push(json!({
                    "asset_type": "other",
                    "count": other_count,
                }));
            }

            // Sort by count in descending order
            result.sort_by(|a, b| {
                let count_a = a.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
                let count_b = b.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
                count_b.cmp(&count_a)
            });

            json!(result)
        }
        _ => json!([]),
    }
}

/// Detects the type of a charm based on its data, matching the webapp's logic
fn detect_charm_type(data: &Value, default_type: &str) -> String {
    // First check if this is a spell with empty data
    if let Some(data_obj) = data.get("data") {
        if data_obj.is_object() && data_obj.as_object().unwrap().is_empty() {
            if let Some(type_value) = data.get("type") {
                if type_value.is_string() && type_value.as_str().unwrap() == "spell" {
                    if let Some(detected) = data.get("detected") {
                        if detected.is_boolean() && detected.as_bool().unwrap() {
                            return "other".to_string(); // Classify empty spells as "other"
                        }
                    }
                }
            }
        }

        // Check for apps in the data.data path (matching webapp logic)
        if let Some(inner_data) = data_obj.get("data") {
            if let Some(apps) = inner_data.get("apps") {
                if apps.is_object() {
                    for (_, app_value) in apps.as_object().unwrap() {
                        if let Some(app_str) = app_value.as_str() {
                            // This is the key logic from the webapp's detectCharmType function
                            if app_str.starts_with("n/") {
                                return "nft".to_string();
                            }
                            if app_str.starts_with("t/") {
                                return "token".to_string();
                            }
                            if app_str.starts_with("d/") {
                                return "dapp".to_string();
                            }
                        }
                    }
                }
            }
        }
    }

    // If we couldn't detect a specific type, return the default
    // This matches the webapp's logic of returning the original asset_type
    if default_type == "nft" || default_type == "token" || default_type == "dapp" {
        default_type.to_string()
    } else {
        "other".to_string()
    }
}

/// Gets recent charms with mempool links for a specific network
async fn get_recent_charms(conn: &DatabaseConnection, network: &str) -> Value {
    let query = format!(
        "
        SELECT txid, charmid, block_height, asset_type, data
        FROM charms 
        WHERE network = '{}' AND NOT (
            data::jsonb -> 'data' = '{{}}'::jsonb AND 
            data::jsonb ->> 'type' = 'spell' AND 
            (data::jsonb ->> 'detected')::boolean = true
        )
        ORDER BY date_created DESC 
        LIMIT 5
    ",
        network
    );

    match conn
        .query_all(Statement::from_string(conn.get_database_backend(), query))
        .await
    {
        Ok(rows) => {
            let result = rows
                .iter()
                .map(|row| {
                    let txid = row.try_get::<String>("", "txid").unwrap_or_default();
                    let charmid = row.try_get::<String>("", "charmid").unwrap_or_default();
                    let block_height = row.try_get::<i32>("", "block_height").unwrap_or(0);
                    let asset_type = row.try_get::<String>("", "asset_type").unwrap_or_default();

                    // Get the data to extract the real charm ID if available
                    let data_str = row.try_get::<String>("", "data").unwrap_or_default();
                    let real_charm_id = if !data_str.is_empty() {
                        if let Ok(data) = serde_json::from_str::<Value>(&data_str) {
                            // Try to extract the real charm ID from the data
                            // First check for data in the standard metadata structure
                            if let Some(data_obj) = data.get("data") {
                                if let Some(id) = data_obj.get("id") {
                                    id.as_str().unwrap_or(&charmid).to_string()
                                } else {
                                    charmid
                                }
                            } else {
                                charmid
                            }
                        } else {
                            charmid
                        }
                    } else {
                        charmid
                    };

                    // Generate mempool link for the transaction based on network
                    let mempool_link = if network == "mainnet" {
                        format!("https://mempool.space/tx/{}", txid)
                    } else {
                        format!("https://mempool.space/testnet4/tx/{}", txid)
                    };

                    // Detect the charm type
                    let detected_type = if !data_str.is_empty() {
                        if let Ok(data) = serde_json::from_str::<Value>(&data_str) {
                            detect_charm_type(&data, &asset_type)
                        } else {
                            asset_type
                        }
                    } else {
                        asset_type
                    };

                    json!({
                        "txid": txid,
                        "mempool_link": mempool_link,
                        "charmid": real_charm_id,
                        "block_height": block_height,
                        "asset_type": detected_type,
                    })
                })
                .collect::<Vec<Value>>();
            json!(result)
        }
        _ => json!([]),
    }
}
