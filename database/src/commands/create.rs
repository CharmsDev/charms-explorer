use sea_orm::{Database, DatabaseConnection, DbErr};
use std::error::Error;
use tracing::{error, info};

use crate::config::DatabaseConfig;

/// Execute the create command
pub async fn execute(_name: Option<String>) -> Result<(), Box<dyn Error>> {
    // Load configuration
    let config = DatabaseConfig::from_env()?;

    info!("Connecting to database: {}", config.url);

    // Try to connect to the database
    match connect_to_database(&config.url).await {
        Ok(_) => {
            info!("Database connection successful");
            Ok(())
        }
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            Err(Box::new(e))
        }
    }
}

async fn connect_to_database(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    Database::connect(database_url).await
}
