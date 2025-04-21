// Configuration Module
// This module handles loading and accessing configuration from environment variables

use dotenv::dotenv;
use std::env;

/// Configuration for the Charms Explorer API
#[derive(Debug, Clone)]
pub struct ApiConfig {
    // Server configuration
    pub host: String,
    pub port: u16,

    // Database configuration
    pub database_url: String,
}

impl ApiConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        // Ensure .env file is loaded
        dotenv().ok();

        // Load configuration from environment variables
        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .unwrap_or(3000);
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://charms:charms@localhost:5432/charms_indexer".to_string()
        });

        Self {
            host,
            port,
            database_url,
        }
    }

    /// Get the server address
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
