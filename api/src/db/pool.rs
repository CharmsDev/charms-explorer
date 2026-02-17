// Database connection pooling management

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;

use crate::config::ApiConfig;
use crate::db::error::DbError;
use crate::db::repositories::Repositories;

/// Database connection pool for managing Sea-ORM connections
pub struct DbPool {
    pool: DatabaseConnection,
}

impl DbPool {
    /// Creates a new database connection pool from API configuration
    pub async fn new(config: &ApiConfig) -> Result<Self, DbError> {
        // Connection pool settings tuned for PgBouncer on Fly.io
        let max_connections = 10;
        let min_connections = 1;
        let connect_timeout = 10;
        let idle_timeout = 30; // Recycle idle connections before PgBouncer kills them
        let acquire_timeout = 5; // Fail fast instead of waiting 10s
        let max_lifetime = 300; // Force reconnect every 5 min
        let debug_mode = false;

        let conn_opts = ConnectOptions::new(config.database_url.clone())
            .max_connections(max_connections)
            .min_connections(min_connections)
            .connect_timeout(Duration::from_secs(connect_timeout))
            .idle_timeout(Duration::from_secs(idle_timeout))
            .acquire_timeout(Duration::from_secs(acquire_timeout))
            .max_lifetime(Duration::from_secs(max_lifetime))
            .sqlx_logging(debug_mode)
            .to_owned();

        Database::connect(conn_opts)
            .await
            .map(|pool| DbPool { pool })
            .map_err(|e| DbError::ConnectionError(e.to_string()))
    }

    /// Returns a reference to the underlying database connection
    #[allow(dead_code)] // Reserved for direct DB access
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.pool
    }

    /// Creates repository instances for database operations
    pub fn repositories(&self) -> Repositories {
        Repositories::new(self.pool.clone())
    }
}
