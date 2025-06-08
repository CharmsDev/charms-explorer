use dotenv::dotenv;
use std::collections::HashMap;
use std::env;

/// Network type enum
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NetworkType {
    /// Bitcoin network
    Bitcoin,
    /// Cardano network
    Cardano,
}

/// Network identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkId {
    /// Network type
    pub network_type: NetworkType,
    /// Network name (e.g., "mainnet", "testnet4")
    pub name: String,
}

impl NetworkId {
    /// Create a new network identifier
    pub fn new(network_type: NetworkType, name: &str) -> Self {
        Self {
            network_type,
            name: name.to_string(),
        }
    }

    /// Get a string representation of the network identifier
    pub fn to_string(&self) -> String {
        format!("{:?}-{}", self.network_type, self.name)
    }

    /// Get the blockchain type as a string
    pub fn blockchain_type(&self) -> String {
        match self.network_type {
            NetworkType::Bitcoin => "Bitcoin".to_string(),
            NetworkType::Cardano => "Cardano".to_string(),
        }
    }
}

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
    /// Network name (e.g., "mainnet", "testnet4")
    pub network: String,
    /// Genesis block height
    pub genesis_block_height: u64,
}

/// Configuration for the Cardano client
#[derive(Debug, Clone)]
pub struct CardanoConfig {
    /// Cardano node URL
    pub url: String,
    /// Network name (e.g., "mainnet", "testnet")
    pub network: String,
    /// Genesis block height
    pub genesis_block_height: u64,
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
    /// Process interval in milliseconds
    pub process_interval_ms: u64,
    /// Number of threads to use for processing
    pub thread_count: usize,
    /// Enable Bitcoin testnet4
    pub enable_bitcoin_testnet4: bool,
    /// Enable Bitcoin mainnet
    pub enable_bitcoin_mainnet: bool,
    /// Enable Cardano networks
    pub enable_cardano: bool,
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Bitcoin client configurations
    pub bitcoin_configs: HashMap<String, BitcoinConfig>,
    /// Cardano client configurations
    pub cardano_configs: HashMap<String, CardanoConfig>,
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

        // Create Bitcoin configurations map
        let mut bitcoin_configs = HashMap::new();

        // Load Bitcoin testnet4 configuration if enabled
        let enable_bitcoin_testnet4 = env::var("ENABLE_BITCOIN_TESTNET4")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        if enable_bitcoin_testnet4 {
            bitcoin_configs.insert(
                "testnet4".to_string(),
                BitcoinConfig {
                    host: env::var("BITCOIN_TESTNET4_RPC_HOST")
                        .expect("BITCOIN_TESTNET4_RPC_HOST environment variable is required"),
                    port: env::var("BITCOIN_TESTNET4_RPC_PORT")
                        .expect("BITCOIN_TESTNET4_RPC_PORT environment variable is required"),
                    username: env::var("BITCOIN_TESTNET4_RPC_USER")
                        .expect("BITCOIN_TESTNET4_RPC_USER environment variable is required"),
                    password: env::var("BITCOIN_TESTNET4_RPC_PASSWORD")
                        .expect("BITCOIN_TESTNET4_RPC_PASSWORD environment variable is required"),
                    network: "testnet4".to_string(),
                    genesis_block_height: env::var("BITCOIN_TESTNET4_GENESIS_BLOCK_HEIGHT")
                        .expect("BITCOIN_TESTNET4_GENESIS_BLOCK_HEIGHT environment variable is required")
                        .parse::<u64>()
                        .expect("BITCOIN_TESTNET4_GENESIS_BLOCK_HEIGHT must be a valid u64"),
                },
            );
        }

        // Load Bitcoin mainnet configuration if enabled
        let enable_bitcoin_mainnet = env::var("ENABLE_BITCOIN_MAINNET")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        if enable_bitcoin_mainnet {
            bitcoin_configs.insert(
                "mainnet".to_string(),
                BitcoinConfig {
                    host: env::var("BITCOIN_MAINNET_RPC_HOST")
                        .expect("BITCOIN_MAINNET_RPC_HOST environment variable is required"),
                    port: env::var("BITCOIN_MAINNET_RPC_PORT")
                        .expect("BITCOIN_MAINNET_RPC_PORT environment variable is required"),
                    username: env::var("BITCOIN_MAINNET_RPC_USER")
                        .expect("BITCOIN_MAINNET_RPC_USER environment variable is required"),
                    password: env::var("BITCOIN_MAINNET_RPC_PASSWORD")
                        .expect("BITCOIN_MAINNET_RPC_PASSWORD environment variable is required"),
                    network: "mainnet".to_string(),
                    genesis_block_height: env::var("BITCOIN_MAINNET_GENESIS_BLOCK_HEIGHT")
                        .expect(
                            "BITCOIN_MAINNET_GENESIS_BLOCK_HEIGHT environment variable is required",
                        )
                        .parse::<u64>()
                        .expect("BITCOIN_MAINNET_GENESIS_BLOCK_HEIGHT must be a valid u64"),
                },
            );
        }

        // Create Cardano configurations map
        let mut cardano_configs = HashMap::new();

        // Load Cardano configurations if enabled
        let enable_cardano = env::var("ENABLE_CARDANO")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        if enable_cardano {
            // Load Cardano mainnet configuration
            if env::var("ENABLE_CARDANO_MAINNET")
                .unwrap_or_else(|_| "true".to_string())
                .parse::<bool>()
                .unwrap_or(true)
            {
                cardano_configs.insert(
                    "mainnet".to_string(),
                    CardanoConfig {
                        url: env::var("CARDANO_MAINNET_URL")
                            .expect("CARDANO_MAINNET_URL environment variable is required"),
                        network: "mainnet".to_string(),
                        genesis_block_height: env::var("CARDANO_MAINNET_GENESIS_BLOCK_HEIGHT")
                            .expect("CARDANO_MAINNET_GENESIS_BLOCK_HEIGHT environment variable is required")
                            .parse::<u64>()
                            .expect("CARDANO_MAINNET_GENESIS_BLOCK_HEIGHT must be a valid u64"),
                    },
                );
            }

            // Load Cardano testnet configuration
            if env::var("ENABLE_CARDANO_TESTNET")
                .unwrap_or_else(|_| "true".to_string())
                .parse::<bool>()
                .unwrap_or(true)
            {
                cardano_configs.insert(
                    "testnet".to_string(),
                    CardanoConfig {
                        url: env::var("CARDANO_TESTNET_URL")
                            .expect("CARDANO_TESTNET_URL environment variable is required"),
                        network: "testnet".to_string(),
                        genesis_block_height: env::var("CARDANO_TESTNET_GENESIS_BLOCK_HEIGHT")
                            .expect("CARDANO_TESTNET_GENESIS_BLOCK_HEIGHT environment variable is required")
                            .parse::<u64>()
                            .expect("CARDANO_TESTNET_GENESIS_BLOCK_HEIGHT must be a valid u64"),
                    },
                );
            }
        }

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
            process_interval_ms: env::var("PROCESS_BLOCK_INTERVAL_MS")
                .expect("PROCESS_BLOCK_INTERVAL_MS environment variable is required")
                .parse::<u64>()
                .expect("PROCESS_BLOCK_INTERVAL_MS must be a valid u64"),
            thread_count: env::var("INDEXER_THREAD_COUNT")
                .unwrap_or_else(|_| "4".to_string())
                .parse::<usize>()
                .expect("INDEXER_THREAD_COUNT must be a valid usize"),
            enable_bitcoin_testnet4,
            enable_bitcoin_mainnet,
            enable_cardano,
        };

        Self {
            bitcoin_configs,
            cardano_configs,
            api: api_config,
            database: database_config,
            indexer: indexer_config,
        }
    }

    /// Get Bitcoin configuration for a specific network
    pub fn get_bitcoin_config(&self, network: &str) -> Option<&BitcoinConfig> {
        self.bitcoin_configs.get(network)
    }

    /// Get Cardano configuration for a specific network
    pub fn get_cardano_config(&self, network: &str) -> Option<&CardanoConfig> {
        self.cardano_configs.get(network)
    }
}
