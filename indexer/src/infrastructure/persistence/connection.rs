use sea_orm::{Database, DatabaseConnection};

use crate::config::AppConfig;
use crate::infrastructure::persistence::error::DbError;
use crate::utils::logging;

/// Manages database connection pool
pub struct DbPool {
    connection: DatabaseConnection,
}

impl DbPool {
    /// Creates a new database connection pool
    pub async fn new(config: &AppConfig) -> Result<Self, DbError> {
        logging::log_info(&format!("Connecting to database: {}", config.database.url));

        match Database::connect(&config.database.url).await {
            Ok(connection) => {
                logging::log_info("Database connection established successfully");
                Ok(DbPool { connection })
            }
            Err(e) => {
                logging::log_error(&format!("Failed to connect to database: {}", e));
                Err(DbError::ConnectionError(format!(
                    "Failed to connect to database: {}",
                    e
                )))
            }
        }
    }

    /// Returns the database connection
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }
}
