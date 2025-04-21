// Database Module
// This module handles database operations for the Charms Explorer API

use sea_orm::{
    ColumnTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};
use std::time::Duration;

use crate::config::ApiConfig;
use crate::entity::charms;

/// Error type for database operations
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Database connection error: {0}")]
    ConnectionError(String),

    #[error("Database query error: {0}")]
    QueryError(String),
}

impl From<sea_orm::DbErr> for DbError {
    fn from(err: sea_orm::DbErr) -> Self {
        DbError::QueryError(err.to_string())
    }
}

/// Database connection pool for managing database connections
pub struct DbPool {
    pool: DatabaseConnection,
}

impl DbPool {
    /// Create a new database connection pool from configuration
    pub async fn new(config: &ApiConfig) -> Result<Self, DbError> {
        // Default connection pool settings
        let max_connections = 10;
        let min_connections = 2;
        let connect_timeout = 10;
        let idle_timeout = 300;
        let debug_mode = false;

        let conn_opts = ConnectOptions::new(config.database_url.clone())
            .max_connections(max_connections)
            .min_connections(min_connections)
            .connect_timeout(Duration::from_secs(connect_timeout))
            .idle_timeout(Duration::from_secs(idle_timeout))
            .sqlx_logging(debug_mode)
            .to_owned();

        Database::connect(conn_opts)
            .await
            .map(|pool| DbPool { pool })
            .map_err(|e| DbError::ConnectionError(e.to_string()))
    }

    /// Get a reference to the database connection
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.pool
    }

    /// Create repositories for database operations
    pub fn repositories(&self) -> Repositories {
        Repositories {
            charm: CharmRepository::new(self.pool.clone()),
        }
    }
}

/// Container for all repositories
pub struct Repositories {
    pub charm: CharmRepository,
}

/// Repository for charm operations
pub struct CharmRepository {
    conn: DatabaseConnection,
}

impl CharmRepository {
    /// Create a new charm repository
    pub fn new(conn: DatabaseConnection) -> Self {
        CharmRepository { conn }
    }

    /// Get a charm by txid
    pub async fn get_by_txid(&self, txid: &str) -> Result<Option<charms::Model>, DbError> {
        charms::Entity::find_by_id(txid.to_string())
            .one(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Find charms by charmid
    pub async fn find_by_charmid(&self, charmid: &str) -> Result<Vec<charms::Model>, DbError> {
        charms::Entity::find()
            .filter(charms::Column::Charmid.eq(charmid))
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Find charms by asset type
    pub async fn find_by_asset_type(
        &self,
        asset_type: &str,
    ) -> Result<Vec<charms::Model>, DbError> {
        charms::Entity::find()
            .filter(charms::Column::AssetType.eq(asset_type))
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Get all charms
    pub async fn get_all(&self) -> Result<Vec<charms::Model>, DbError> {
        charms::Entity::find()
            .order_by_desc(charms::Column::BlockHeight)
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Get all charm numbers by asset type
    pub async fn get_charm_numbers_by_type(
        &self,
        asset_type: Option<&str>,
    ) -> Result<Vec<String>, DbError> {
        let mut query = charms::Entity::find();

        if let Some(asset_type) = asset_type {
            query = query.filter(charms::Column::AssetType.eq(asset_type));
        }

        let charms = query
            .order_by_desc(charms::Column::BlockHeight)
            .all(&self.conn)
            .await?;

        // Extract charmid from each charm
        let charm_numbers = charms.into_iter().map(|c| c.charmid).collect();

        Ok(charm_numbers)
    }
}
