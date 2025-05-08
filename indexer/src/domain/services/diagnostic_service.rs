// Indexer diagnostic service implementation

use bitcoincore_rpc::{Auth, Client, RpcApi};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::infrastructure::persistence::repositories::BookmarkRepository;

/// Service for indexer diagnostics
#[derive(Clone)]
pub struct DiagnosticService {
    conn: DatabaseConnection,
    bookmark_repository: BookmarkRepository,
}

impl DiagnosticService {
    /// Creates a new diagnostic service with database connection
    pub fn new(conn: &DatabaseConnection, bookmark_repository: BookmarkRepository) -> Self {
        Self {
            conn: conn.clone(),
            bookmark_repository,
        }
    }

    /// Performs a comprehensive indexer diagnostic check
    pub async fn diagnose(&self) -> Value {
        let mut result = HashMap::new();

        // Check database connection
        let connection_info = self.get_connection_info().await;
        result.insert("connection", connection_info);

        // Test Bitcoin RPC connection
        let bitcoin_rpc_test = self.test_bitcoin_rpc_connection().await;
        result.insert("bitcoin_rpc_test", bitcoin_rpc_test);

        // Get indexer status
        let indexer_status = self.get_indexer_status().await;
        result.insert("indexer_status", indexer_status);

        // Get charm statistics
        let charm_stats = self.get_charm_statistics().await;
        result.insert("charm_stats", charm_stats);

        json!(result)
    }

    /// Gets database connection information
    async fn get_connection_info(&self) -> Value {
        let backend = match self.conn.get_database_backend() {
            DbBackend::Postgres => "PostgreSQL",
            DbBackend::MySql => "MySQL",
            DbBackend::Sqlite => "SQLite",
        };

        // Try to get database version
        let version = match self
            .conn
            .query_one(Statement::from_string(
                self.conn.get_database_backend(),
                match self.conn.get_database_backend() {
                    DbBackend::Postgres => "SELECT version();",
                    DbBackend::MySql => "SELECT version();",
                    DbBackend::Sqlite => "SELECT sqlite_version();",
                }
                .to_string(),
            ))
            .await
        {
            Ok(Some(row)) => row.try_get::<String>("", "version").unwrap_or_else(|_| {
                // Try to get the first column regardless of its name
                row.try_get_by_index::<String>(0).unwrap_or_default()
            }),
            _ => "Unknown".to_string(),
        };

        json!({
            "backend": backend,
            "version": version,
            "status": "connected"
        })
    }

    /// Tests the Bitcoin RPC connection
    async fn test_bitcoin_rpc_connection(&self) -> Value {
        // Use Fly.io Bitcoin RPC connection details
        let host = "bitcoind-t4-test.fly.dev".to_string();
        let port = "48332".to_string();
        let username = "hello".to_string();
        let password = "world".to_string();

        let rpc_url = format!("http://{}:{}", host, port);
        let auth = Auth::UserPass(username, password);

        // Try to connect to the Bitcoin RPC server
        match Client::new(&rpc_url, auth) {
            Ok(client) => {
                // Try to get the block count
                match client.get_block_count() {
                    Ok(block_count) => {
                        // If block count succeeded, try to get the best block hash
                        let best_block_hash = match client.get_best_block_hash() {
                            Ok(hash) => hash.to_string(),
                            Err(_) => "Unknown".to_string(),
                        };

                        json!({
                            "status": "connected",
                            "host": host,
                            "port": port,
                            "block_count": block_count,
                            "best_block_hash": best_block_hash,
                        })
                    }
                    Err(e) => {
                        json!({
                            "status": "error",
                            "host": host,
                            "port": port,
                            "error": format!("Failed to get block count: {}", e),
                            "suggestion": "Check if the Bitcoin node is running and accessible."
                        })
                    }
                }
            }
            Err(e) => {
                json!({
                    "status": "error",
                    "host": host,
                    "port": port,
                    "error": format!("Failed to connect to Bitcoin RPC: {}", e),
                    "suggestion": "Check the Bitcoin RPC connection details and ensure the node is accessible."
                })
            }
        }
    }

    /// Gets the current status of the indexer
    async fn get_indexer_status(&self) -> Value {
        // Get the last processed block
        let last_block = match self.bookmark_repository.get_last_processed_block().await {
            Ok(Some(height)) => height,
            Ok(None) => 0,
            Err(_) => 0,
        };

        // Get the last updated timestamp
        let last_updated = match self.bookmark_repository.get_last_updated_timestamp().await {
            Ok(Some(timestamp)) => timestamp.to_rfc3339(),
            Ok(None) => "Never".to_string(),
            Err(_) => "Error".to_string(),
        };

        // Get the latest confirmed block
        let latest_confirmed_block = self.get_latest_confirmed_block().await;

        // Calculate time since last update
        let time_since_update = match self.bookmark_repository.get_last_updated_timestamp().await {
            Ok(Some(timestamp)) => {
                let now = chrono::Utc::now();
                let duration = now.signed_duration_since(timestamp);
                format!("{} seconds", duration.num_seconds())
            }
            Ok(None) => "Never".to_string(),
            Err(_) => "Error".to_string(),
        };

        json!({
            "last_processed_block": last_block,
            "last_updated_at": last_updated,
            "latest_confirmed_block": latest_confirmed_block,
            "time_since_last_update": time_since_update,
            "status": if time_since_update == "Never" || time_since_update == "Error" {
                "unknown"
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
        })
    }

    /// Gets the latest confirmed block
    async fn get_latest_confirmed_block(&self) -> Value {
        let query = "
            SELECT height 
            FROM bookmark 
            WHERE status = 'confirmed' 
            ORDER BY height DESC 
            LIMIT 1
        ";

        match self
            .conn
            .query_one(Statement::from_string(
                self.conn.get_database_backend(),
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
    async fn get_charm_statistics(&self) -> Value {
        // Get total number of charms
        let total_charms = self.count_table_rows("charms").await;

        // Get total number of transactions
        let total_transactions = self.count_table_rows("transactions").await;

        // Get confirmed transactions count
        let confirmed_transactions = self.count_confirmed_transactions().await;

        // Get charms by asset type
        let charms_by_asset_type = self.get_charms_by_asset_type().await;

        // Get recent charms
        let recent_charms = self.get_recent_charms().await;

        json!({
            "total_charms": total_charms,
            "total_transactions": total_transactions,
            "confirmed_transactions": confirmed_transactions,
            "charms_by_asset_type": charms_by_asset_type,
            "recent_charms": recent_charms
        })
    }

    /// Counts rows in a table
    async fn count_table_rows(&self, table: &str) -> i64 {
        let query = format!("SELECT COUNT(*) as count FROM {}", table);

        match self
            .conn
            .query_one(Statement::from_string(
                self.conn.get_database_backend(),
                query,
            ))
            .await
        {
            Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(0),
            _ => 0,
        }
    }

    /// Counts confirmed transactions
    async fn count_confirmed_transactions(&self) -> i64 {
        let query = "SELECT COUNT(*) as count FROM transactions WHERE status = 'confirmed'";

        match self
            .conn
            .query_one(Statement::from_string(
                self.conn.get_database_backend(),
                query.to_string(),
            ))
            .await
        {
            Ok(Some(row)) => row.try_get::<i64>("", "count").unwrap_or(0),
            _ => 0,
        }
    }

    /// Gets charms grouped by asset type
    async fn get_charms_by_asset_type(&self) -> Value {
        let query = "
            SELECT asset_type, COUNT(*) as count 
            FROM charms 
            GROUP BY asset_type 
            ORDER BY count DESC
        ";

        match self
            .conn
            .query_all(Statement::from_string(
                self.conn.get_database_backend(),
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
    async fn get_recent_charms(&self) -> Value {
        let query = "
            SELECT txid, charmid, block_height, asset_type, date_created
            FROM charms 
            ORDER BY date_created DESC 
            LIMIT 5
        ";

        match self
            .conn
            .query_all(Statement::from_string(
                self.conn.get_database_backend(),
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
}
