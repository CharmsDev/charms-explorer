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

    // Bitcoin RPC configuration
    pub bitcoin_rpc_host: String,
    pub bitcoin_rpc_port: String,
    pub bitcoin_rpc_username: String,
    pub bitcoin_rpc_password: String,
}

impl ApiConfig {
    /// Creates configuration instance from required environment variables
    pub fn from_env() -> Self {
        dotenv().ok();

        let host = env::var("HOST").expect("HOST environment variable must be set");
        let port = env::var("PORT")
            .expect("PORT environment variable must be set")
            .parse::<u16>()
            .expect("PORT must be a valid port number");
        let database_url =
            env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");

        // Bitcoin RPC configuration - required environment variables
        let bitcoin_rpc_host = env::var("BITCOIN_RPC_HOST")
            .expect("BITCOIN_RPC_HOST environment variable must be set");
        let bitcoin_rpc_port = env::var("BITCOIN_RPC_PORT")
            .expect("BITCOIN_RPC_PORT environment variable must be set");
        let bitcoin_rpc_username = env::var("BITCOIN_RPC_USERNAME")
            .expect("BITCOIN_RPC_USERNAME environment variable must be set");
        let bitcoin_rpc_password = env::var("BITCOIN_RPC_PASSWORD")
            .expect("BITCOIN_RPC_PASSWORD environment variable must be set");

        Self {
            host,
            port,
            database_url,
            bitcoin_rpc_host,
            bitcoin_rpc_port,
            bitcoin_rpc_username,
            bitcoin_rpc_password,
        }
    }

    /// Returns formatted server address string (host:port)
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
