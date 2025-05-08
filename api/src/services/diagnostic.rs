// Database diagnostic service implementation

use bitcoincore_rpc::{Auth, Client, RpcApi};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;

/// Service for database diagnostics
pub struct DiagnosticService {
    conn: DatabaseConnection,
}

impl DiagnosticService {
    /// Creates a new diagnostic service with database connection
    pub fn new(conn: &DatabaseConnection) -> Self {
        Self { conn: conn.clone() }
    }

    /// Performs a comprehensive database diagnostic check
    pub async fn diagnose(&self) -> Value {
        let mut result = HashMap::new();

        // Check database connection
        let connection_info = self.get_connection_info().await;
        result.insert("connection", connection_info);

        // Check if required tables exist and create them if they don't
        let tables_check = self.check_and_create_tables().await;
        result.insert("tables_check", tables_check);

        // List database tables
        let tables = self.list_tables().await;
        result.insert("tables", tables);

        // Get table structures
        let table_structures = self.get_table_structures().await;
        result.insert("table_structures", table_structures);

        // Run a test query
        let test_query = self.run_test_query().await;
        result.insert("test_query", test_query);

        // Test Bitcoin RPC connection
        let bitcoin_rpc_test = self.test_bitcoin_rpc_connection().await;
        result.insert("bitcoin_rpc_test", bitcoin_rpc_test);

        json!(result)
    }

    /// Checks if required tables exist and creates them if they don't
    async fn check_and_create_tables(&self) -> Value {
        let required_tables = vec!["bookmark", "charms", "transactions"];
        let mut missing_tables = Vec::new();
        let mut created_tables = Vec::new();
        let mut errors = Vec::new();

        // Check which tables are missing
        for table in &required_tables {
            let exists = self.table_exists(table).await;
            if !exists {
                missing_tables.push(table.to_string());
            }
        }

        // If there are missing tables, create them
        if !missing_tables.is_empty() {
            for table in &missing_tables {
                let result = self.create_table(table).await;
                match result {
                    Ok(_) => created_tables.push(table.to_string()),
                    Err(e) => errors.push(format!("Failed to create table {}: {}", table, e)),
                }
            }
        }

        json!({
            "required_tables": required_tables,
            "missing_tables": missing_tables,
            "created_tables": created_tables,
            "errors": errors
        })
    }

    /// Checks if a table exists in the database
    async fn table_exists(&self, table_name: &str) -> bool {
        let query = match self.conn.get_database_backend() {
            DbBackend::Postgres => format!(
                "SELECT EXISTS (
                    SELECT FROM information_schema.tables 
                    WHERE table_schema = 'public' 
                    AND table_name = '{}'
                )",
                table_name
            ),
            DbBackend::MySql => format!(
                "SELECT EXISTS (
                    SELECT FROM information_schema.tables 
                    WHERE table_schema = DATABASE() 
                    AND table_name = '{}'
                )",
                table_name
            ),
            DbBackend::Sqlite => format!(
                "SELECT EXISTS (
                    SELECT name FROM sqlite_master 
                    WHERE type='table' 
                    AND name='{}'
                )",
                table_name
            ),
        };

        match self
            .conn
            .query_one(Statement::from_string(
                self.conn.get_database_backend(),
                query,
            ))
            .await
        {
            Ok(Some(row)) => {
                // Try to get the first column regardless of its name
                row.try_get_by_index::<bool>(0).unwrap_or(false)
            }
            _ => false,
        }
    }

    /// Creates a table in the database
    async fn create_table(&self, table_name: &str) -> Result<(), String> {
        let sql = match table_name {
            "bookmark" => {
                "
                CREATE TABLE bookmark (
                    hash CHARACTER VARYING PRIMARY KEY,
                    height INTEGER NOT NULL,
                    status CHARACTER VARYING NOT NULL DEFAULT 'pending'
                );
                CREATE INDEX IF NOT EXISTS bookmark_height ON bookmark(height);
            "
            }
            "charms" => {
                "
                CREATE TABLE charms (
                    txid CHARACTER VARYING PRIMARY KEY,
                    charmid CHARACTER VARYING NOT NULL,
                    block_height INTEGER NOT NULL,
                    data JSONB NOT NULL DEFAULT '{}',
                    date_created TIMESTAMP NOT NULL DEFAULT NOW(),
                    asset_type CHARACTER VARYING NOT NULL
                );
                CREATE INDEX IF NOT EXISTS charms_block_height ON charms(block_height);
                CREATE INDEX IF NOT EXISTS charms_asset_type ON charms(asset_type);
                CREATE INDEX IF NOT EXISTS charms_charmid ON charms(charmid);
            "
            }
            "transactions" => {
                "
                CREATE TABLE transactions (
                    txid CHARACTER VARYING PRIMARY KEY,
                    block_height INTEGER NOT NULL,
                    ordinal BIGINT NOT NULL,
                    raw JSONB NOT NULL DEFAULT '{}',
                    charm JSONB NOT NULL DEFAULT '{}',
                    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
                    status CHARACTER VARYING NOT NULL DEFAULT 'pending',
                    confirmations INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS transactions_block_height ON transactions(block_height);
            "
            }
            _ => return Err(format!("Unknown table: {}", table_name)),
        };

        // Split SQL statements by semicolon and execute each one separately
        let statements: Vec<&str> = sql
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for stmt in statements {
            match self
                .conn
                .execute(Statement::from_string(
                    self.conn.get_database_backend(),
                    format!("{};", stmt),
                ))
                .await
            {
                Ok(_) => continue,
                Err(e) => return Err(format!("Database error: {}", e)),
            }
        }

        Ok(())
    }

    /// Gets database connection information
    async fn get_connection_info(&self) -> Value {
        let backend = match self.conn.get_database_backend() {
            DbBackend::Postgres => "PostgreSQL",
            DbBackend::MySql => "MySQL",
            DbBackend::Sqlite => "SQLite",
            _ => "Unknown",
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

    /// Lists all tables in the database
    async fn list_tables(&self) -> Value {
        let query = match self.conn.get_database_backend() {
            DbBackend::Postgres => {
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'"
            }
            DbBackend::MySql => {
                "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE()"
            }
            DbBackend::Sqlite => "SELECT name FROM sqlite_master WHERE type='table'",
            _ => "SELECT table_name FROM information_schema.tables",
        };

        let tables = match self
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

                rows.iter()
                    .filter_map(|row| row.try_get::<String>("", column_name).ok())
                    .collect::<Vec<String>>()
            }
            Err(_) => vec!["Error listing tables".to_string()],
        };

        json!(tables)
    }

    /// Gets structure information for all tables
    async fn get_table_structures(&self) -> Value {
        let tables = match self
            .conn
            .query_all(Statement::from_string(
                self.conn.get_database_backend(),
                match self.conn.get_database_backend() {
                    DbBackend::Postgres => {
                        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'"
                    }
                    DbBackend::MySql => {
                        "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE()"
                    }
                    DbBackend::Sqlite => "SELECT name FROM sqlite_master WHERE type='table'",
                }
                .to_string(),
            ))
            .await
        {
            Ok(rows) => {
                let column_name = match self.conn.get_database_backend() {
                    DbBackend::Sqlite => "name",
                    _ => "table_name",
                };

                rows.iter()
                    .filter_map(|row| row.try_get::<String>("", column_name).ok())
                    .collect::<Vec<String>>()
            }
            Err(_) => vec![],
        };

        let mut structures = HashMap::new();

        for table in tables {
            let columns = self.get_table_columns(&table).await;
            structures.insert(table, columns);
        }

        json!(structures)
    }

    /// Gets column information for a specific table
    async fn get_table_columns(&self, table: &str) -> Value {
        let query = match self.conn.get_database_backend() {
            DbBackend::Postgres => format!(
                "SELECT column_name, data_type, is_nullable 
                 FROM information_schema.columns 
                 WHERE table_name = '{}'",
                table
            ),
            DbBackend::MySql => format!(
                "SELECT column_name, data_type, is_nullable 
                 FROM information_schema.columns 
                 WHERE table_name = '{}'",
                table
            ),
            DbBackend::Sqlite => format!("PRAGMA table_info({})", table),
        };

        match self
            .conn
            .query_all(Statement::from_string(
                self.conn.get_database_backend(),
                query,
            ))
            .await
        {
            Ok(rows) => {
                if self.conn.get_database_backend() == DbBackend::Sqlite {
                    // SQLite PRAGMA table_info returns different column names
                    let columns = rows
                        .iter()
                        .map(|row| {
                            json!({
                                "name": row.try_get::<String>("", "name").unwrap_or_default(),
                                "type": row.try_get::<String>("", "type").unwrap_or_default(),
                                "notnull": row.try_get::<i32>("", "notnull").unwrap_or_default(),
                                "pk": row.try_get::<i32>("", "pk").unwrap_or_default(),
                            })
                        })
                        .collect::<Vec<Value>>();
                    json!(columns)
                } else {
                    // PostgreSQL and MySQL
                    let columns = rows
                        .iter()
                        .map(|row| {
                            json!({
                                "name": row.try_get::<String>("", "column_name").unwrap_or_default(),
                                "type": row.try_get::<String>("", "data_type").unwrap_or_default(),
                                "nullable": row.try_get::<String>("", "is_nullable").unwrap_or_default(),
                            })
                        })
                        .collect::<Vec<Value>>();
                    json!(columns)
                }
            }
            Err(_) => json!([{"error": format!("Failed to get columns for table {}", table)}]),
        }
    }

    /// Tests the Bitcoin RPC connection
    async fn test_bitcoin_rpc_connection(&self) -> Value {
        // Use Fly.io Bitcoin RPC connection details
        let host = "bitcoind-t4-test.fly.dev".to_string();
        let port = "48332".to_string();
        let username = "hello".to_string();
        let password = "world".to_string();

        // Local testing connection details (commented out)
        // let host = "localhost".to_string();

        let rpc_url = format!("http://{}:{}", host, port);
        let auth = Auth::UserPass(username, password);

        // Try to connect to the Bitcoin RPC server
        match Client::new(&rpc_url, auth) {
            Ok(client) => {
                // Try to get the block count instead of blockchain info
                // This avoids issues with network name validation
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

    /// Runs a test query to check database functionality
    async fn run_test_query(&self) -> Value {
        // Try to query the charms table
        let charms_query = self
            .conn
            .query_all(Statement::from_string(
                self.conn.get_database_backend(),
                "SELECT COUNT(*) as count FROM charms".to_string(),
            ))
            .await;

        let charms_count = match charms_query {
            Ok(rows) => {
                if let Some(row) = rows.first() {
                    row.try_get::<i64>("", "count").unwrap_or(-1)
                } else {
                    -1
                }
            }
            Err(e) => {
                return json!({
                    "error": format!("Error querying charms table: {}", e),
                    "suggestion": "The 'charms' table might not exist. Check the database schema and migrations."
                });
            }
        };

        json!({
            "charms_count": charms_count
        })
    }
}
