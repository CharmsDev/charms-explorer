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
            host: env::var("BITCOIN_RPC_HOST")
                .expect("BITCOIN_RPC_HOST environment variable is required"),
            port: env::var("BITCOIN_RPC_PORT")
                .expect("BITCOIN_RPC_PORT environment variable is required"),
            username: env::var("BITCOIN_RPC_USER")
                .expect("BITCOIN_RPC_USER environment variable is required"),
            password: env::var("BITCOIN_RPC_PASSWORD")
                .expect("BITCOIN_RPC_PASSWORD environment variable is required"),
        };

        // Load API configuration
        let api_config = ApiConfig {
            url: env::var("CHARMS_API_URL")
                .expect("CHARMS_API_URL environment variable is required"),
        };

        // Load database configuration
        let database_config = DatabaseConfig {
            url: env::var("DATABASE_URL").expect("DATABASE_URL environment variable is required"),
        };

        // Load indexer configuration
        let indexer_config = IndexerConfig {
            genesis_block_height: env::var("GENESIS_BLOCK_HEIGHT")
                .expect("GENESIS_BLOCK_HEIGHT environment variable is required")
                .parse::<u64>()
                .expect("GENESIS_BLOCK_HEIGHT must be a valid u64"),
            process_interval_ms: env::var("PROCESS_BLOCK_INTERVAL_MS")
                .expect("PROCESS_BLOCK_INTERVAL_MS environment variable is required")
                .parse::<u64>()
                .expect("PROCESS_BLOCK_INTERVAL_MS must be a valid u64"),
        };

        Self {
            bitcoin: bitcoin_config,
            api: api_config,
            database: database_config,
            indexer: indexer_config,
        }
    }
}
