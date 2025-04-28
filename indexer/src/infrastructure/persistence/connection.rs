use sea_orm::{Database, DatabaseConnection};

use crate::config::AppConfig;
use crate::infrastructure::persistence::error::DbError;

/// Database connection pool
pub struct DbPool {
    connection: DatabaseConnection,
}

impl DbPool {
    /// Create a new database connection pool
    pub async fn new(config: &AppConfig) -> Result<Self, DbError> {
        match Database::connect(&config.database.url).await {
            Ok(connection) => Ok(DbPool { connection }),
            Err(e) => Err(DbError::ConnectionError(format!(
                "Failed to connect to database: {}",
                e
            ))),
        }
    }

    /// Get the database connection
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }
}
