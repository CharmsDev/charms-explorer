use dotenv::dotenv;
use std::env;

/// Configuration for the Bitcoin client
#[derive(Debug, Clone)]
pub struct BitcoinConfig {
    /// Bitcoin RPC host
    pub host: String,
    /// Bitcoin RPC port
    pub port: String,
    /// Bitcoin RPC username
    pub username: String,
    /// Bitcoin RPC password
    pub password: String,
}

/// Configuration for the API client
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// API URL
    pub url: String,
}

/// Configuration for the database
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,
}

/// Configuration for the indexer
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    /// Genesis block height
    pub genesis_block_height: u64,
    /// Process interval in milliseconds
    pub process_interval_ms: u64,
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Bitcoin client configuration
    pub bitcoin: BitcoinConfig,
    /// API client configuration
    pub api: ApiConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Indexer configuration
    pub indexer: IndexerConfig,
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        // Ensure .env file is loaded
        dotenv().ok();

        // Load Bitcoin configuration
        let bitcoin_config = BitcoinConfig {
            host: env::var("BITCOIN_RPC_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: env::var("BITCOIN_RPC_PORT").unwrap_or_else(|_| "48332".to_string()),
            username: env::var("BITCOIN_RPC_USER").unwrap_or_else(|_| "hello".to_string()),
            password: env::var("BITCOIN_RPC_PASSWORD").unwrap_or_else(|_| "world".to_string()),
        };

        // Load API configuration
        let api_config = ApiConfig {
            url: env::var("CHARMS_API_URL")
                .unwrap_or_else(|_| "https://api-t4.charms.dev".to_string()),
        };

        // Load database configuration
        let database_config = DatabaseConfig {
            url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://charms:charms@localhost:5432/charms_indexer".to_string()
            }),
        };

        // Load indexer configuration
        let indexer_config = IndexerConfig {
            genesis_block_height: env::var("GENESIS_BLOCK_HEIGHT")
                .unwrap_or_else(|_| "57604".to_string())
                .parse::<u64>()
                .unwrap_or(57604),
            process_interval_ms: env::var("PROCESS_BLOCK_INTERVAL_MS")
                .unwrap_or_else(|_| "120000".to_string())
                .parse::<u64>()
                .unwrap_or(120000),
        };

        Self {
            bitcoin: bitcoin_config,
            api: api_config,
            database: database_config,
            indexer: indexer_config,
        }
    }
}
