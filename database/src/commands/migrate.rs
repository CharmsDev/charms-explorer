use sea_orm::Database;
use sea_orm_migration::prelude::*;
use std::error::Error;
use tracing::{error, info};

use crate::config::DatabaseConfig;
use crate::migration;

/// Execute the migrate command
pub async fn execute(steps: Option<u32>) -> Result<(), Box<dyn Error>> {
    // Load configuration
    let config = DatabaseConfig::from_env()?;

    info!("Connecting to database: {}", config.url);

    // Connect to the database
    let connection = match Database::connect(&config.url).await {
        Ok(conn) => conn,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            return Err(Box::new(e));
        }
    };

    info!("Running migrations...");

    // Log migration files to be applied
    let all_migrations = migration::Migrator::get_migration_files();
    info!("Found {} migration files", all_migrations.len());
    for m in &all_migrations {
        info!("Migration file: {}", m.name());
    }

    // Run migrations
    let _schema_manager = SchemaManager::new(&connection);

    // Run migrations
    match steps {
        Some(n) => {
            info!("Running {} migrations", n);
            match migration::Migrator::up(&connection, Some(n)).await {
                Ok(_) => {
                    info!("Successfully ran {} migrations", n);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to run migrations: {}", e);
                    error!("Migration error details: {:?}", e);
                    Err(Box::new(e))
                }
            }
        }
        None => {
            info!("Running all pending migrations");
            match migration::Migrator::up(&connection, None).await {
                Ok(_) => {
                    info!("Successfully ran all migrations");
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to run migrations: {}", e);
                    error!("Migration error details: {:?}", e);
                    Err(Box::new(e))
                }
            }
        }
    }
}
