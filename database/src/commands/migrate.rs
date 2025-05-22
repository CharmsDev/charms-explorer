use sea_orm::Database;
use sea_orm_migration::prelude::*;
use std::error::Error;
use tracing::{info, error};

use crate::config::DatabaseConfig;

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
    
    // Run migrations
    let _schema_manager = SchemaManager::new(&connection);
    
    // Get migrator from the migration crate
    let _migrator = migration::Migrator;
    
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
                    Err(Box::new(e))
                }
            }
        }
    }
}

/// Reset the database (drop all tables and run migrations)
pub async fn reset() -> Result<(), Box<dyn Error>> {
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
    
    info!("Resetting database...");
    
    // Get migrator from the migration crate
    let _migrator = migration::Migrator;
    
    // Reset database
    match migration::Migrator::fresh(&connection).await {
        Ok(_) => {
            info!("Successfully reset database");
            Ok(())
        }
        Err(e) => {
            error!("Failed to reset database: {}", e);
            Err(Box::new(e))
        }
    }
}

/// Show database migration status
pub async fn status() -> Result<(), Box<dyn Error>> {
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
    
    info!("Checking migration status...");
    
    // Get migrator from the migration crate
    let migrator = migration::Migrator;
    
    // Get migration status
    let schema_manager = SchemaManager::new(&connection);
    
    // Check if migration table exists
    match schema_manager.has_table("seaql_migrations").await {
        Ok(exists) => {
            if !exists {
                info!("Migration table does not exist. No migrations have been run.");
                return Ok(());
            }
        }
        Err(e) => {
            error!("Failed to check migration table: {}", e);
            return Err(Box::new(e));
        }
    }
    
    // Get applied migrations
    let applied_migrations = match migration::Migrator::get_applied_migrations(&connection).await {
        Ok(migrations) => migrations,
        Err(e) => {
            error!("Failed to get applied migrations: {}", e);
            return Err(Box::new(e));
        }
    };
    
    // Get all migrations
    let all_migrations = migration::Migrator::get_migration_files();
    
    // Print status
    info!("Migration status:");
    info!("Applied migrations: {}", applied_migrations.len());
    info!("Total migrations: {}", all_migrations.len());
    
    // Print pending migrations
    let pending_count = all_migrations.len() - applied_migrations.len();
    if pending_count > 0 {
        info!("Pending migrations: {}", pending_count);
        
        // Get names of applied migrations
        let applied_names: Vec<String> = applied_migrations
            .iter()
            .map(|m| m.name().to_string())
            .collect();
        
        // Print pending migration names
        info!("  - {} pending migrations", pending_count);
    } else {
        info!("No pending migrations");
    }
    
    Ok(())
}
