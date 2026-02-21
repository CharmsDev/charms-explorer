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
        // Connection pool — tuned for high-volume production (100 users × 10 addresses = 1000 rps)
        //
        // Fly.io Postgres (via PgBouncer in transaction mode) supports up to ~100 server-side
        // connections. We keep our pool at 50 to leave headroom for the indexer's own pool.
        // Each request holds a connection only for the duration of the query (~1-5ms), so
        // 50 connections can serve thousands of rps with proper async queuing.
        let max_connections: u32 = std::env::var("DB_POOL_MAX")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);
        let min_connections: u32 = std::env::var("DB_POOL_MIN")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        // How long to wait for a free connection before returning 503
        let acquire_timeout_secs: u64 = 8;
        // Recycle idle connections before PgBouncer's server_idle_timeout kills them
        let idle_timeout_secs: u64 = 25;
        // Force full reconnect periodically to avoid stale connections
        let max_lifetime_secs: u64 = 300;
        let connect_timeout_secs: u64 = 10;
        let debug_mode = false;

        let conn_opts = ConnectOptions::new(config.database_url.clone())
            .max_connections(max_connections)
            .min_connections(min_connections)
            .connect_timeout(Duration::from_secs(connect_timeout_secs))
            .idle_timeout(Duration::from_secs(idle_timeout_secs))
            .acquire_timeout(Duration::from_secs(acquire_timeout_secs))
            .max_lifetime(Duration::from_secs(max_lifetime_secs))
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
