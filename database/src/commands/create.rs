use std::error::Error;
use std::process::Command;
use tracing::{info, error};

use crate::config::DatabaseConfig;

/// Execute the create command
pub async fn execute(name: Option<String>) -> Result<(), Box<dyn Error>> {
    // Load configuration
    let config = DatabaseConfig::from_env()?;
    
    // Use provided name or default from config
    let db_name = name.unwrap_or(config.name);
    
    info!("Creating database: {}", db_name);
    
    // Build connection string for postgres command
    let conn_string = format!(
        "postgresql://{}:{}@{}:{}",
        config.user, config.password, config.host, config.port
    );
    
    // Check if database exists
    let check_output = Command::new("psql")
        .arg(&conn_string)
        .arg("-c")
        .arg(format!("SELECT 1 FROM pg_database WHERE datname = '{}'", db_name))
        .arg("-t")
        .output();
    
    match check_output {
        Ok(output) => {
            let exists = String::from_utf8_lossy(&output.stdout).trim().eq("1");
            
            if exists {
                info!("Database '{}' already exists", db_name);
                return Ok(());
            }
            
            // Create database
            let create_output = Command::new("psql")
                .arg(&conn_string)
                .arg("-c")
                .arg(format!("CREATE DATABASE {}", db_name))
                .output();
            
            match create_output {
                Ok(output) => {
                    if output.status.success() {
                        info!("Database '{}' created successfully", db_name);
                        Ok(())
                    } else {
                        let error_msg = String::from_utf8_lossy(&output.stderr);
                        error!("Failed to create database: {}", error_msg);
                        Err(format!("Failed to create database: {}", error_msg).into())
                    }
                }
                Err(e) => {
                    error!("Failed to execute psql command: {}", e);
                    Err(Box::new(e))
                }
            }
        }
        Err(e) => {
            error!("Failed to check if database exists: {}", e);
            Err(Box::new(e))
        }
    }
}
