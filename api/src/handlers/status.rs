// Indexer status endpoint handler implementation

use axum::{extract::State, response::IntoResponse, Json};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::db::repositories::Repositories;

/// Handler for GET /status - Returns the indexer status
pub async fn get_indexer_status(
    State(repositories): State<Arc<Repositories>>,
) -> impl IntoResponse {
    let conn = repositories.charm.get_connection();
    let status = get_status(conn).await;
    Json(status)
}

/// Gets the current status of the indexer
async fn get_status(conn: &DatabaseConnection) -> Value {
    // Get the last processed block
    let last_block = match get_last_processed_block(conn).await {
        Ok(Some(height)) => height,
        Ok(None) => 0,
        Err(_) => 0,
    };

    // Get the latest confirmed block
    let latest_confirmed_block = get_latest_confirmed_block(conn).await;

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

    // Calculate time since last update - try multiple sources
    let time_since_update = match get_last_updated_timestamp(conn).await {
        Ok(Some(timestamp)) => {
            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(timestamp);
            format!("{} seconds", duration.num_seconds())
        }
        Ok(None) => {
            // If no transactions, try to get timestamp from charms
            match get_latest_charm_timestamp(conn).await {
                Ok(Some(timestamp)) => {
                    let now = chrono::Utc::now();
                    let duration = now.signed_duration_since(timestamp);
                    format!("{} seconds", duration.num_seconds())
                }
                Ok(None) => "Never".to_string(),
                Err(_) => "Error".to_string(),
            }
        }
        Err(_) => "Error".to_string(),
    };

    // Determine status based on available data
    let status = if total_charms > 0 {
        if time_since_update == "Never" || time_since_update == "Error" {
            "indexed" // We have charms but no recent updates
        } else if time_since_update.starts_with("Error") {
            "error"
        } else {
            let seconds = time_since_update
                .split_whitespace()
                .next()
                .unwrap_or("0")
                .parse::<i64>()
                .unwrap_or(0);

            if seconds < 60 {
                "active"
            } else if seconds < 300 {
                "idle"
            } else {
                "inactive"
            }
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
        "time_since_last_update": time_since_update,
        "status": status,
        "charm_stats": charm_stats
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

/// Gets statistics about indexed charms
async fn get_charm_statistics(conn: &DatabaseConnection) -> Value {
    // Get total number of charms
    let total_charms = count_table_rows(conn, "charms").await;

    // Get total number of transactions
    let total_transactions = count_table_rows(conn, "transactions").await;

    // Get confirmed transactions count
    let confirmed_transactions = count_confirmed_transactions(conn).await;

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

/// Counts rows in a table
async fn count_table_rows(conn: &DatabaseConnection, table: &str) -> i64 {
    let query = format!("SELECT COUNT(*) as count FROM {}", table);

    match conn
        .query_one(Statement::from_string(conn.get_database_backend(), query))
        .await
    {
        Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(0),
        _ => 0,
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

/// Gets charms grouped by asset type
async fn get_charms_by_asset_type(conn: &DatabaseConnection) -> Value {
    let query = "
        SELECT asset_type, COUNT(*) as count 
        FROM charms 
        GROUP BY asset_type 
        ORDER BY count DESC
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
                    json!({
                        "asset_type": row.try_get::<String>("", "asset_type").unwrap_or_default(),
                        "count": row.try_get::<i64>("", "count").unwrap_or(0),
                    })
                })
                .collect::<Vec<Value>>();
            json!(result)
        }
        _ => json!([]),
    }
}

/// Gets recent charms
async fn get_recent_charms(conn: &DatabaseConnection) -> Value {
    let query = "
        SELECT txid, charmid, block_height, asset_type, date_created
        FROM charms 
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
                    json!({
                        "txid": row.try_get::<String>("", "txid").unwrap_or_default(),
                        "charmid": row.try_get::<String>("", "charmid").unwrap_or_default(),
                        "block_height": row.try_get::<i32>("", "block_height").unwrap_or(0),
                        "asset_type": row.try_get::<String>("", "asset_type").unwrap_or_default(),
                        "date_created": row.try_get::<chrono::DateTime<chrono::Utc>>("", "date_created")
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_default(),
                    })
                })
                .collect::<Vec<Value>>();
            json!(result)
        }
        _ => json!([]),
    }
}
