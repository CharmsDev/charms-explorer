// Configuration management from environment variables

use dotenv::dotenv;
use std::env;

/// Configuration settings for the Charms Explorer API server
#[derive(Debug, Clone)]
pub struct ApiConfig {
    // Server configuration
    pub host: String,
    pub port: u16,

    // Database configuration
    pub database_url: String,
}

impl ApiConfig {
    /// Creates configuration instance from environment variables with defaults
    pub fn from_env() -> Self {
        dotenv().ok();

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

    /// Returns formatted server address string (host:port)
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
