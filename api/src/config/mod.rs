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
    #[allow(dead_code)] // Reserved for network switching
    pub enable_bitcoin_testnet4: bool,
    #[allow(dead_code)] // Reserved for network switching
    pub enable_bitcoin_mainnet: bool,
    #[allow(dead_code)] // Reserved for network switching
    pub enable_cardano: bool,

    // Esplora REST base URLs (mempool.space by default — free, no key)
    pub bitcoin_mainnet_esplora_url: String,
    pub bitcoin_testnet4_esplora_url: String,
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

        // Network configuration.
        // Defaults: mainnet ON, testnet4 OFF. testnet4 code paths remain in
        // the binary but require an explicit env override to activate.
        let enable_bitcoin_testnet4 = env::var("ENABLE_BITCOIN_TESTNET4")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);
        let enable_bitcoin_mainnet = env::var("ENABLE_BITCOIN_MAINNET")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);
        let enable_cardano = env::var("ENABLE_CARDANO")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        // Esplora (mempool.space) base URLs, overridable per network
        let bitcoin_mainnet_esplora_url = env::var("BITCOIN_MAINNET_ESPLORA_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| crate::services::mempool_space_service::DEFAULT_MAINNET_URL.to_string());
        let bitcoin_testnet4_esplora_url = env::var("BITCOIN_TESTNET4_ESPLORA_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| crate::services::mempool_space_service::DEFAULT_TESTNET4_URL.to_string());

        Self {
            host,
            port,
            database_url,
            enable_bitcoin_testnet4,
            enable_bitcoin_mainnet,
            enable_cardano,
            bitcoin_mainnet_esplora_url,
            bitcoin_testnet4_esplora_url,
        }
    }

    /// Returns formatted server address string (host:port)
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
