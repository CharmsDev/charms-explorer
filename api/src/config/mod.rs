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

    // Network configuration
    pub enable_bitcoin_testnet4: bool,
    pub enable_bitcoin_mainnet: bool,
    pub enable_cardano: bool,

    // Bitcoin Testnet4 RPC configuration
    pub bitcoin_testnet4_rpc_host: String,
    pub bitcoin_testnet4_rpc_port: String,
    pub bitcoin_testnet4_rpc_username: String,
    pub bitcoin_testnet4_rpc_password: String,

    // Bitcoin Mainnet RPC configuration
    pub bitcoin_mainnet_rpc_host: String,
    pub bitcoin_mainnet_rpc_port: String,
    pub bitcoin_mainnet_rpc_username: String,
    pub bitcoin_mainnet_rpc_password: String,
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

        // Network configuration
        let enable_bitcoin_testnet4 = env::var("ENABLE_BITCOIN_TESTNET4")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);
        let enable_bitcoin_mainnet = env::var("ENABLE_BITCOIN_MAINNET")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);
        let enable_cardano = env::var("ENABLE_CARDANO")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        // Bitcoin Testnet4 RPC configuration
        let bitcoin_testnet4_rpc_host =
            env::var("BITCOIN_TESTNET4_RPC_HOST").unwrap_or_else(|_| "localhost".to_string());
        let bitcoin_testnet4_rpc_port =
            env::var("BITCOIN_TESTNET4_RPC_PORT").unwrap_or_else(|_| "48332".to_string());
        let bitcoin_testnet4_rpc_username =
            env::var("BITCOIN_TESTNET4_RPC_USERNAME").unwrap_or_else(|_| "hello".to_string());
        let bitcoin_testnet4_rpc_password =
            env::var("BITCOIN_TESTNET4_RPC_PASSWORD").unwrap_or_else(|_| "world".to_string());

        // Bitcoin Mainnet RPC configuration
        let bitcoin_mainnet_rpc_host =
            env::var("BITCOIN_MAINNET_RPC_HOST").unwrap_or_else(|_| "localhost".to_string());
        let bitcoin_mainnet_rpc_port =
            env::var("BITCOIN_MAINNET_RPC_PORT").unwrap_or_else(|_| "8332".to_string());
        let bitcoin_mainnet_rpc_username =
            env::var("BITCOIN_MAINNET_RPC_USERNAME").unwrap_or_else(|_| "bitcoinrpc".to_string());
        let bitcoin_mainnet_rpc_password =
            env::var("BITCOIN_MAINNET_RPC_PASSWORD").unwrap_or_else(|_| "password".to_string());

        Self {
            host,
            port,
            database_url,
            enable_bitcoin_testnet4,
            enable_bitcoin_mainnet,
            enable_cardano,
            bitcoin_testnet4_rpc_host,
            bitcoin_testnet4_rpc_port,
            bitcoin_testnet4_rpc_username,
            bitcoin_testnet4_rpc_password,
            bitcoin_mainnet_rpc_host,
            bitcoin_mainnet_rpc_port,
            bitcoin_mainnet_rpc_username,
            bitcoin_mainnet_rpc_password,
        }
    }

    /// Returns formatted server address string (host:port)
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
