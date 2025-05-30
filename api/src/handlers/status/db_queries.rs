// Database query utilities for status handler

use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde_json::{json, Value};

/// Gets the last processed block height from the bookmark table for a specific network
pub async fn get_last_processed_block(
    conn: &DatabaseConnection,
    network: &str,
) -> Result<Option<i32>, String> {
    let query = format!(
        "
        SELECT height 
        FROM bookmark 
        WHERE network = '{}'
        ORDER BY height DESC 
        LIMIT 1
    ",
        network
    );

    match conn
        .query_one(Statement::from_string(conn.get_database_backend(), query))
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

/// Gets the last updated timestamp from the transactions table for a specific network
pub async fn get_last_updated_timestamp(
    conn: &DatabaseConnection,
    network: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, String> {
    let query = format!(
        "
        SELECT updated_at 
        FROM transactions 
        WHERE network = '{}'
        ORDER BY updated_at DESC 
        LIMIT 1
    ",
        network
    );

    match conn
        .query_one(Statement::from_string(conn.get_database_backend(), query))
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

/// Gets the latest timestamp from the charms table for a specific network
pub async fn get_latest_charm_timestamp(
    conn: &DatabaseConnection,
    network: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, String> {
    let query = format!(
        "
        SELECT date_created 
        FROM charms 
        WHERE network = '{}'
        ORDER BY date_created DESC 
        LIMIT 1
    ",
        network
    );

    match conn
        .query_one(Statement::from_string(conn.get_database_backend(), query))
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

/// Gets the latest confirmed block height for a specific network
pub async fn get_latest_confirmed_block(conn: &DatabaseConnection, network: &str) -> Value {
    let query = format!(
        "
        SELECT height 
        FROM bookmark 
        WHERE status = 'confirmed' AND network = '{}'
        ORDER BY height DESC 
        LIMIT 1
    ",
        network
    );

    match conn
        .query_one(Statement::from_string(conn.get_database_backend(), query))
        .await
    {
        Ok(Some(row)) => {
            let height = row.try_get::<i32>("", "height").unwrap_or(0);
            json!(height)
        }
        _ => json!(0),
    }
}

/// Counts rows in a table with network filter
pub async fn count_table_rows(conn: &DatabaseConnection, table: &str, network: &str) -> i64 {
    // For the charms table, we need to filter out empty spell charms
    if table == "charms" {
        // First try a simpler query to get all charms for the network
        let all_charms_query = format!(
            "SELECT COUNT(*) as count FROM charms WHERE network = '{}'",
            network
        );
        let all_charms_count = match conn
            .query_one(Statement::from_string(
                conn.get_database_backend(),
                all_charms_query,
            ))
            .await
        {
            Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(0),
            _ => 0,
        };

        // If we have charms, try to filter out empty spell charms
        if all_charms_count > 0 {
            // Try a more specific query to filter out empty spell charms
            let query = format!(
                "
                SELECT COUNT(*) as count 
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
                .query_one(Statement::from_string(conn.get_database_backend(), query))
                .await
            {
                Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(all_charms_count),
                _ => all_charms_count, // Fallback to all charms if the filter query fails
            }
        } else {
            0 // No charms at all
        }
    } else {
        // For other tables, use the standard count with network filter
        let query = format!(
            "SELECT COUNT(*) as count FROM {} WHERE network = '{}'",
            table, network
        );

        match conn
            .query_one(Statement::from_string(conn.get_database_backend(), query))
            .await
        {
            Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(0),
            _ => 0,
        }
    }
}

/// Counts transactions associated with valid charms for a specific network
pub async fn count_valid_transactions(conn: &DatabaseConnection, network: &str) -> i64 {
    let query = format!(
        "
        SELECT COUNT(DISTINCT t.txid) as count
        FROM transactions t
        JOIN charms c ON t.txid = c.txid
        WHERE t.network = '{}' AND c.network = '{}' AND NOT (
            c.data::jsonb -> 'data' = '{{}}'::jsonb AND 
            c.data::jsonb ->> 'type' = 'spell' AND 
            (c.data::jsonb ->> 'detected')::boolean = true
        )
    ",
        network, network
    );

    match conn
        .query_one(Statement::from_string(conn.get_database_backend(), query))
        .await
    {
        Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(0),
        _ => 0,
    }
}

/// Counts confirmed transactions associated with valid charms for a specific network
pub async fn count_confirmed_valid_transactions(conn: &DatabaseConnection, network: &str) -> i64 {
    let query = format!(
        "
        SELECT COUNT(DISTINCT t.txid) as count
        FROM transactions t
        JOIN charms c ON t.txid = c.txid
        WHERE t.status = 'confirmed' AND t.network = '{}' AND c.network = '{}' AND NOT (
            c.data::jsonb -> 'data' = '{{}}'::jsonb AND 
            c.data::jsonb ->> 'type' = 'spell' AND 
            (c.data::jsonb ->> 'detected')::boolean = true
        )
    ",
        network, network
    );

    match conn
        .query_one(Statement::from_string(conn.get_database_backend(), query))
        .await
    {
        Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(0),
        _ => 0,
    }
}

/// Gets recent processed blocks with charm counts for a specific network
pub async fn get_recent_processed_blocks(conn: &DatabaseConnection, network: &str) -> Value {
    let query = format!(
        "
        SELECT b.height, b.hash, b.status, 
        COUNT(CASE WHEN c.txid IS NOT NULL AND NOT (
            c.data::jsonb -> 'data' = '{{}}'::jsonb AND 
            c.data::jsonb ->> 'type' = 'spell' AND 
            (c.data::jsonb ->> 'detected')::boolean = true
        ) THEN 1 ELSE NULL END) as charm_count
        FROM bookmark b
        LEFT JOIN charms c ON b.height = c.block_height AND c.network = '{}'
        WHERE b.network = '{}'
        GROUP BY b.height, b.hash, b.status
        ORDER BY b.height DESC
        LIMIT 10
    ",
        network, network
    );

    match conn
        .query_all(Statement::from_string(conn.get_database_backend(), query))
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

/// Calculate time since last update for status determination
pub async fn calculate_time_since_update(conn: &DatabaseConnection, network: &str) -> i64 {
    match get_last_updated_timestamp(conn, network).await {
        Ok(Some(timestamp)) => {
            let now = Utc::now();
            let duration = now.signed_duration_since(timestamp);
            duration.num_seconds()
        }
        Ok(None) => -1, // No timestamp available
        Err(_) => -2,   // Error getting timestamp
    }
}
