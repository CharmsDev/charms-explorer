// Database diagnostic service implementation

use bitcoincore_rpc::{Auth, Client, RpcApi};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::config::ApiConfig;

/// Service for database diagnostics
pub struct DiagnosticService {
    conn: DatabaseConnection,
    config: ApiConfig,
}

impl DiagnosticService {
    /// Creates a new diagnostic service with database connection and configuration
    pub fn new(conn: &DatabaseConnection, config: &ApiConfig) -> Self {
        Self {
            conn: conn.clone(),
            config: config.clone(),
        }
    }

    /// Performs a comprehensive database diagnostic check
    pub async fn diagnose(&self) -> Value {
        let mut result = HashMap::new();

        // Get database connection information
        let db_info = self.get_database_info().await;
        result.insert("db_connection", db_info);

        // Get detailed table information with counts
        let tables_info = self.get_tables_with_counts().await;
        result.insert("tables", tables_info);

        // Get summary table content
        let summary_content = self.get_summary_table_content().await;
        result.insert("summary_table", summary_content);

        // Test Bitcoin RPC connection
        let bitcoin_rpc_test = self.test_bitcoin_rpc_connection().await;
        result.insert("bitcoin_rpc", bitcoin_rpc_test);

        json!(result)
    }

    /// Gets the content of the summary table
    async fn get_summary_table_content(&self) -> Value {
        let query = "SELECT * FROM summary";

        match self
            .conn
            .query_all(Statement::from_string(
                self.conn.get_database_backend(),
                query.to_string(),
            ))
            .await
        {
            Ok(rows) => {
                if rows.is_empty() {
                    return json!({
                        "status": "warning",
                        "message": "Summary table exists but is empty",
                        "rows": []
                    });
                }

                let mut summary_rows = Vec::new();
                for row in rows {
                    let mut row_data = HashMap::new();
                    
                    // Extract all fields from the row
                    if let Ok(id) = row.try_get::<i32>("", "id") {
                        row_data.insert("id", json!(id));
                    }
                    
                    if let Ok(network) = row.try_get::<String>("", "network") {
                        row_data.insert("network", json!(network));
                    }
                    
                    if let Ok(last_processed_block) = row.try_get::<i32>("", "last_processed_block") {
                        row_data.insert("last_processed_block", json!(last_processed_block));
                    }
                    
                    if let Ok(latest_confirmed_block) = row.try_get::<i32>("", "latest_confirmed_block") {
                        row_data.insert("latest_confirmed_block", json!(latest_confirmed_block));
                    }
                    
                    if let Ok(total_charms) = row.try_get::<i64>("", "total_charms") {
                        row_data.insert("total_charms", json!(total_charms));
                    }
                    
                    if let Ok(bitcoin_node_status) = row.try_get::<String>("", "bitcoin_node_status") {
                        row_data.insert("bitcoin_node_status", json!(bitcoin_node_status));
                    }
                    
                    summary_rows.push(json!(row_data));
                }

                json!({
                    "status": "success",
                    "count": summary_rows.len(),
                    "rows": summary_rows
                })
            }
            Err(e) => {
                json!({
                    "status": "error",
                    "message": format!("Failed to query summary table: {}", e),
                    "rows": []
                })
            }
        }
    }

    /// Gets database connection information
    async fn get_database_info(&self) -> Value {
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

        // Get database name
        let db_name = match self
            .conn
            .query_one(Statement::from_string(
                self.conn.get_database_backend(),
                match self.conn.get_database_backend() {
                    DbBackend::Postgres => "SELECT current_database();",
                    DbBackend::MySql => "SELECT DATABASE();",
                    DbBackend::Sqlite => "PRAGMA database_list;",
                }
                .to_string(),
            ))
            .await
        {
            Ok(Some(row)) => {
                if self.conn.get_database_backend() == DbBackend::Sqlite {
                    row.try_get::<String>("", "file").unwrap_or_default()
                } else {
                    row.try_get_by_index::<String>(0).unwrap_or_default()
                }
            },
            _ => "Unknown".to_string(),
        };

        json!({
            "type": backend,
            "version": version,
            "database_name": db_name,
            "status": "connected"
        })
    }

    /// Gets all tables with their row counts
    async fn get_tables_with_counts(&self) -> Value {
        // Get list of tables
        let tables_result = self.list_tables().await;

        match tables_result {
            Ok(tables) => {
                if tables.is_empty() {
                    return json!({
                        "status": "warning",
                        "message": "No tables found in database",
                        "tables": []
                    });
                }

                let mut table_info = Vec::new();

                // For each table, get the row count
                for table in tables {
                    let count = self.get_table_count(&table).await;
                    table_info.push(json!({
                        "name": table,
                        "row_count": count
                    }));
                }

                json!({
                    "status": "success",
                    "count": table_info.len(),
                    "tables": table_info
                })
            }
            Err(err) => {
                json!({
                    "status": "error",
                    "message": format!("Failed to list tables: {}", err),
                    "tables": []
                })
            }
        }
    }

    /// Lists all tables in the database
    async fn list_tables(&self) -> Result<Vec<String>, String> {
        // First, try to directly list the expected tables we know should exist
        let expected_tables = vec![
            "bookmark".to_string(),
            "charms".to_string(),
            "transactions".to_string(),
            "summary".to_string(),
            "seaql_migrations".to_string(),
        ];

        // Check if we can access at least one of the expected tables
        for table in &expected_tables {
            let test_query = format!("SELECT 1 FROM {} LIMIT 1", table);
            match self
                .conn
                .query_one(Statement::from_string(
                    self.conn.get_database_backend(),
                    test_query,
                ))
                .await
            {
                Ok(_) => {
                    // If we can access at least one table, return the list of expected tables
                    // This is a fallback in case the information_schema queries don't work
                    return Ok(expected_tables.clone());
                }
                Err(_) => continue,
            }
        }

        // If direct table access failed, try the standard approach
        let query = match self.conn.get_database_backend() {
            DbBackend::Postgres => {
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'"
            }
            DbBackend::MySql => {
                "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE()"
            }
            DbBackend::Sqlite => "SELECT name FROM sqlite_master WHERE type='table'",
        };

        match self
            .conn
            .query_all(Statement::from_string(
                self.conn.get_database_backend(),
                query.to_string(),
            ))
            .await
        {
            Ok(rows) => {
                let column_name = match self.conn.get_database_backend() {
                    DbBackend::Sqlite => "name",
                    _ => "table_name",
                };

                let tables = rows
                    .iter()
                    .filter_map(|row| row.try_get::<String>("", column_name).ok())
                    .collect::<Vec<String>>();

                if tables.is_empty() {
                    // If we still got no tables, try a more direct approach for Postgres
                    if self.conn.get_database_backend() == DbBackend::Postgres {
                        match self
                            .conn
                            .query_all(Statement::from_string(
                                DbBackend::Postgres,
                                "SELECT tablename FROM pg_tables WHERE schemaname = 'public'"
                                    .to_string(),
                            ))
                            .await
                        {
                            Ok(pg_rows) => {
                                let pg_tables = pg_rows
                                    .iter()
                                    .filter_map(|row| row.try_get::<String>("", "tablename").ok())
                                    .collect::<Vec<String>>();

                                if !pg_tables.is_empty() {
                                    return Ok(pg_tables);
                                }
                            }
                            Err(e) => return Err(format!("Failed to query pg_tables: {}", e)),
                        }
                    }

                    // Last resort: return the expected tables anyway
                    return Ok(expected_tables.clone());
                }

                Ok(tables)
            }
            Err(e) => Err(format!("Failed to query tables: {}", e)),
        }
    }

    /// Gets row count for a specific table
    async fn get_table_count(&self, table: &str) -> i64 {
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

    /// Tests the Bitcoin RPC connection
    async fn test_bitcoin_rpc_connection(&self) -> Value {
        // Use Bitcoin Testnet4 RPC connection details from configuration
        let host = &self.config.bitcoin_testnet4_rpc_host;
        let port = &self.config.bitcoin_testnet4_rpc_port;
        let username = &self.config.bitcoin_testnet4_rpc_username;
        let password = &self.config.bitcoin_testnet4_rpc_password;

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
                            Err(_) => "Unknown".to_string(),
                        };

                        // Try to get network info to determine if mainnet or testnet
                        let network = "testnet"; // Default to testnet for now

                        json!({
                            "status": "connected",
                            "block_count": block_count,
                            "best_block_hash": best_block_hash,
                            "network": network,
                        })
                    }
                    Ok(Err(e)) => {
                        json!({
                            "status": "error",
                            "error": format!("Failed to get block count: {}", e)
                        })
                    }
                    Err(_) => {
                        json!({
                            "status": "timeout",
                            "error": "Bitcoin RPC request timed out after 5 seconds"
                        })
                    }
                }
            }
            Err(e) => {
                json!({
                    "status": "error",
                    "error": format!("Failed to connect to Bitcoin RPC: {}", e)
                })
            }
        }
    }
}
