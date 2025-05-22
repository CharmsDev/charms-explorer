// Indexer status endpoint handler implementation

use axum::{extract::State, response::IntoResponse, Json};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde_json::{json, Value};

use crate::handlers::AppState;

/// Handler for GET /status - Returns the indexer status
pub async fn get_indexer_status(State(app_state): State<AppState>) -> impl IntoResponse {
    let conn = app_state.repositories.charm.get_connection();
    let status = get_status(conn, &app_state.config).await;
    Json(status)
}

/// Gets the current status of the indexer
async fn get_status(conn: &DatabaseConnection, config: &crate::config::ApiConfig) -> Value {
    // Get the last processed block
    let last_block = match get_last_processed_block(conn).await {
        Ok(Some(height)) => height,
        Ok(None) => 0,
        Err(_) => 0,
    };

    // Get the latest confirmed block
    let latest_confirmed_block = get_latest_confirmed_block(conn).await;

    // Get Bitcoin node information
    let bitcoin_node_info = get_bitcoin_node_info(config).await;

    // Get recent processed blocks with charm counts
    let recent_blocks = get_recent_processed_blocks(conn).await;

    // Get charm statistics
    let charm_stats = get_charm_statistics(conn).await;
    let total_charms = charm_stats
        .get("total_charms")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    // Get the last updated timestamp - try multiple sources
    let last_updated = match get_last_updated_timestamp(conn).await {
        Ok(Some(timestamp)) => timestamp.to_rfc3339(),
        Ok(None) => {
            // If no transactions, try to get timestamp from charms
            match get_latest_charm_timestamp(conn).await {
                Ok(Some(timestamp)) => timestamp.to_rfc3339(),
                Ok(None) => "Never".to_string(),
                Err(_) => "Error".to_string(),
            }
        }
        Err(_) => "Error".to_string(),
    };

    // Get the last indexer loop time
    let last_indexer_loop_time = match get_last_updated_timestamp(conn).await {
        Ok(Some(timestamp)) => timestamp.to_rfc3339(),
        Ok(None) => {
            // If no transactions, try to get timestamp from charms
            match get_latest_charm_timestamp(conn).await {
                Ok(Some(timestamp)) => timestamp.to_rfc3339(),
                Ok(None) => "Never".to_string(),
                Err(_) => "Error".to_string(),
            }
        }
        Err(_) => "Error".to_string(),
    };

    // Calculate time since last update for status determination
    let time_since_update_seconds = match get_last_updated_timestamp(conn).await {
        Ok(Some(timestamp)) => {
            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(timestamp);
            duration.num_seconds()
        }
        Ok(None) => -1, // No timestamp available
        Err(_) => -2,   // Error getting timestamp
    };

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
        "last_processed_block": last_block,
        "last_updated_at": last_updated,
        "latest_confirmed_block": latest_confirmed_block,
        "last_indexer_loop_time": last_indexer_loop_time,
        "status": status,
        "bitcoin_node": bitcoin_node_info,
        "charm_stats": charm_stats,
        "recent_blocks": recent_blocks
    })
}

/// Gets the last processed block height from the bookmark table
async fn get_last_processed_block(conn: &DatabaseConnection) -> Result<Option<i32>, String> {
    let query = "
        SELECT height 
        FROM bookmark 
        ORDER BY height DESC 
        LIMIT 1
    ";

    match conn
        .query_one(Statement::from_string(
            conn.get_database_backend(),
            query.to_string(),
        ))
        .await
    {
        Ok(Some(row)) => {
            let height = row.try_get::<i32>("", "height").unwrap_or(0);
            Ok(Some(height))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(format!("Database error: {}", e)),
    }
}

/// Gets the last updated timestamp from the transactions table
async fn get_last_updated_timestamp(
    conn: &DatabaseConnection,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, String> {
    // Try to get the most recent updated_at timestamp from the transactions table
    let query = "
        SELECT updated_at 
        FROM transactions 
        ORDER BY updated_at DESC 
        LIMIT 1
    ";

    match conn
        .query_one(Statement::from_string(
            conn.get_database_backend(),
            query.to_string(),
        ))
        .await
    {
        Ok(Some(row)) => match row.try_get::<chrono::DateTime<chrono::Utc>>("", "updated_at") {
            Ok(timestamp) => Ok(Some(timestamp)),
            Err(_) => Ok(None),
        },
        Ok(None) => Ok(None),
        Err(e) => Err(format!("Database error: {}", e)),
    }
}

/// Gets the latest timestamp from the charms table
async fn get_latest_charm_timestamp(
    conn: &DatabaseConnection,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, String> {
    let query = "
        SELECT date_created 
        FROM charms 
        ORDER BY date_created DESC 
        LIMIT 1
    ";

    match conn
        .query_one(Statement::from_string(
            conn.get_database_backend(),
            query.to_string(),
        ))
        .await
    {
        Ok(Some(row)) => match row.try_get::<chrono::DateTime<chrono::Utc>>("", "date_created") {
            Ok(timestamp) => Ok(Some(timestamp)),
            Err(_) => Ok(None),
        },
        Ok(None) => Ok(None),
        Err(e) => Err(format!("Database error: {}", e)),
    }
}

/// Gets the latest confirmed block height
async fn get_latest_confirmed_block(conn: &DatabaseConnection) -> Value {
    let query = "
        SELECT height 
        FROM bookmark 
        WHERE status = 'confirmed' 
        ORDER BY height DESC 
        LIMIT 1
    ";

    match conn
        .query_one(Statement::from_string(
            conn.get_database_backend(),
            query.to_string(),
        ))
        .await
    {
        Ok(Some(row)) => {
            let height = row.try_get::<i32>("", "height").unwrap_or(0);
            json!(height)
        }
        _ => json!(0),
    }
}

/// Gets Bitcoin node information by directly connecting to the Bitcoin RPC
async fn get_bitcoin_node_info(config: &crate::config::ApiConfig) -> Value {
    use bitcoincore_rpc::{Auth, Client, RpcApi};

    // Use Bitcoin RPC connection details from configuration
    let host = &config.bitcoin_rpc_host;
    let port = &config.bitcoin_rpc_port;
    let username = &config.bitcoin_rpc_username;
    let password = &config.bitcoin_rpc_password;

    let rpc_url = format!("http://{}:{}", host, port);
    let auth = Auth::UserPass(username.clone(), password.clone());

    // Try to connect to the Bitcoin RPC server
    match Client::new(&rpc_url, auth) {
        Ok(client) => {
            // Try to get the block count with a timeout to prevent hanging
            let block_count_result =
                tokio::time::timeout(std::time::Duration::from_secs(5), async {
                    client.get_block_count()
                })
                .await;

            match block_count_result {
                Ok(Ok(block_count)) => {
                    // If block count succeeded, try to get the best block hash
                    let best_block_hash = match client.get_best_block_hash() {
                        Ok(hash) => hash.to_string(),
                        Err(_) => "unknown".to_string(),
                    };

                    // Try to get network info to determine if mainnet or testnet
                    let network = "testnet"; // Default to testnet for now

                    json!({
                        "status": "connected",
                        "network": network,
                        "block_count": block_count,
                        "best_block_hash": best_block_hash
                    })
                }
                Ok(Err(e)) => {
                    tracing::error!("Failed to get block count: {}", e);
                    json!({
                        "status": "error",
                        "network": "testnet",
                        "block_count": 0,
                        "best_block_hash": "unknown"
                    })
                }
                Err(_) => {
                    tracing::error!("Bitcoin RPC request timed out after 5 seconds");
                    json!({
                        "status": "timeout",
                        "network": "testnet",
                        "block_count": 0,
                        "best_block_hash": "unknown"
                    })
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to connect to Bitcoin RPC: {}", e);
            json!({
                "status": "error",
                "network": "testnet",
                "block_count": 0,
                "best_block_hash": "unknown"
            })
        }
    }
}

/// Gets statistics about indexed charms
async fn get_charm_statistics(conn: &DatabaseConnection) -> Value {
    // Get total number of charms
    let total_charms = count_table_rows(conn, "charms").await;

    // Get total number of transactions associated with valid charms
    let total_transactions = count_valid_transactions(conn).await;

    // Get confirmed transactions count (only for valid charms)
    let confirmed_transactions = count_confirmed_valid_transactions(conn).await;

    // Calculate confirmation rate
    let confirmation_rate = if total_transactions > 0 {
        (confirmed_transactions as f64 / total_transactions as f64 * 100.0).round() as i64
    } else {
        0
    };

    // Get charms by asset type
    let charms_by_asset_type = get_charms_by_asset_type(conn).await;

    // Get recent charms
    let recent_charms = get_recent_charms(conn).await;

    json!({
        "total_charms": total_charms,
        "total_transactions": total_transactions,
        "confirmed_transactions": confirmed_transactions,
        "confirmation_rate": confirmation_rate,
        "charms_by_asset_type": charms_by_asset_type,
        "recent_charms": recent_charms
    })
}

/// Counts transactions associated with valid charms
async fn count_valid_transactions(conn: &DatabaseConnection) -> i64 {
    let query = "
        SELECT COUNT(DISTINCT t.txid) as count
        FROM transactions t
        JOIN charms c ON t.txid = c.txid
        WHERE NOT (
            c.data::jsonb -> 'data' = '{}'::jsonb AND 
            c.data::jsonb ->> 'type' = 'spell' AND 
            (c.data::jsonb ->> 'detected')::boolean = true
        )
    ";

    match conn
        .query_one(Statement::from_string(
            conn.get_database_backend(),
            query.to_string(),
        ))
        .await
    {
        Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(0),
        _ => 0,
    }
}

/// Counts confirmed transactions associated with valid charms
async fn count_confirmed_valid_transactions(conn: &DatabaseConnection) -> i64 {
    let query = "
        SELECT COUNT(DISTINCT t.txid) as count
        FROM transactions t
        JOIN charms c ON t.txid = c.txid
        WHERE t.status = 'confirmed'
        AND NOT (
            c.data::jsonb -> 'data' = '{}'::jsonb AND 
            c.data::jsonb ->> 'type' = 'spell' AND 
            (c.data::jsonb ->> 'detected')::boolean = true
        )
    ";

    match conn
        .query_one(Statement::from_string(
            conn.get_database_backend(),
            query.to_string(),
        ))
        .await
    {
        Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(0),
        _ => 0,
    }
}

/// Counts rows in a table
async fn count_table_rows(conn: &DatabaseConnection, table: &str) -> i64 {
    // For the charms table, we need to filter out empty spell charms
    if table == "charms" {
        // First try a simpler query to get all charms
        let all_charms_query = "SELECT COUNT(*) as count FROM charms";
        let all_charms_count = match conn
            .query_one(Statement::from_string(
                conn.get_database_backend(),
                all_charms_query.to_string(),
            ))
            .await
        {
            Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(0),
            _ => 0,
        };

        // If we have charms, try to filter out empty spell charms
        if all_charms_count > 0 {
            // Try a more specific query to filter out empty spell charms
            let query = "
                SELECT COUNT(*) as count 
                FROM charms 
                WHERE NOT (
                    data::jsonb -> 'data' = '{}'::jsonb AND 
                    data::jsonb ->> 'type' = 'spell' AND 
                    (data::jsonb ->> 'detected')::boolean = true
                )
            ";

            match conn
                .query_one(Statement::from_string(
                    conn.get_database_backend(),
                    query.to_string(),
                ))
                .await
            {
                Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(all_charms_count),
                _ => all_charms_count, // Fallback to all charms if the filter query fails
            }
        } else {
            0 // No charms at all
        }
    } else {
        // For other tables, use the standard count
        let query = format!("SELECT COUNT(*) as count FROM {}", table);

        match conn
            .query_one(Statement::from_string(conn.get_database_backend(), query))
            .await
        {
            Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(0),
            _ => 0,
        }
    }
}

/// Counts confirmed transactions
async fn count_confirmed_transactions(conn: &DatabaseConnection) -> i64 {
    let query = "SELECT COUNT(*) as count FROM transactions WHERE status = 'confirmed'";

    match conn
        .query_one(Statement::from_string(
            conn.get_database_backend(),
            query.to_string(),
        ))
        .await
    {
        Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(0),
        _ => 0,
    }
}

/// Gets charms grouped by asset type using improved backend detection logic
async fn get_charms_by_asset_type(conn: &DatabaseConnection) -> Value {
    // First, get all charms with their data, excluding empty spell charms
    let query = "
        SELECT txid, charmid, data, asset_type
        FROM charms
        WHERE NOT (
            data::jsonb -> 'data' = '{}'::jsonb AND 
            data::jsonb ->> 'type' = 'spell' AND 
            (data::jsonb ->> 'detected')::boolean = true
        )
    ";

    match conn
        .query_all(Statement::from_string(
            conn.get_database_backend(),
            query.to_string(),
        ))
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

/// Gets recent processed blocks with charm counts
async fn get_recent_processed_blocks(conn: &DatabaseConnection) -> Value {
    let query = "
        SELECT b.height, b.hash, b.status, 
        COUNT(CASE WHEN c.txid IS NOT NULL AND NOT (
            c.data::jsonb -> 'data' = '{}'::jsonb AND 
            c.data::jsonb ->> 'type' = 'spell' AND 
            (c.data::jsonb ->> 'detected')::boolean = true
        ) THEN 1 ELSE NULL END) as charm_count
        FROM bookmark b
        LEFT JOIN charms c ON b.height = c.block_height
        GROUP BY b.height, b.hash, b.status
        ORDER BY b.height DESC
        LIMIT 10
    ";

    match conn
        .query_all(Statement::from_string(
            conn.get_database_backend(),
            query.to_string(),
        ))
        .await
    {
        Ok(rows) => {
            let result = rows
                .iter()
                .map(|row| {
                    let height = row.try_get::<i32>("", "height").unwrap_or(0);
                    let hash = row.try_get::<String>("", "hash").unwrap_or_default();
                    let status = row.try_get::<String>("", "status").unwrap_or_default();
                    let charm_count = row.try_get::<i64>("", "charm_count").unwrap_or(0);

                    json!({
                        "height": height,
                        "hash": hash,
                        "status": status,
                        "charm_count": charm_count
                    })
                })
                .collect::<Vec<Value>>();
            json!(result)
        }
        _ => json!([]),
    }
}

/// Gets recent charms with mempool links
async fn get_recent_charms(conn: &DatabaseConnection) -> Value {
    let query = "
        SELECT txid, charmid, block_height, asset_type, data
        FROM charms 
        WHERE NOT (
            data::jsonb -> 'data' = '{}'::jsonb AND 
            data::jsonb ->> 'type' = 'spell' AND 
            (data::jsonb ->> 'detected')::boolean = true
        )
        ORDER BY date_created DESC 
        LIMIT 5
    ";

    match conn
        .query_all(Statement::from_string(
            conn.get_database_backend(),
            query.to_string(),
        ))
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

                    // Generate mempool link for the transaction
                    let mempool_link = format!("https://mempool.space/testnet4/tx/{}", txid);

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
