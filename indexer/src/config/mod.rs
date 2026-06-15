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

    /// Get the blockchain type as a string
    pub fn blockchain_type(&self) -> String {
        match self.network_type {
            NetworkType::Bitcoin => "Bitcoin".to_string(),
            NetworkType::Cardano => "Cardano".to_string(),
        }
    }
}

impl std::fmt::Display for NetworkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}-{}", self.network_type, self.name)
    }
}

/// Provider type for Bitcoin networks
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderType {
    QuickNode,
    BitcoinNode,
}

impl ProviderType {
    /// Parse a provider name (case-insensitive). Unknown values default to
    /// `BitcoinNode`. Not exposed as `FromStr` because parsing is infallible.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "quicknode" => ProviderType::QuickNode,
            "bitcoin_node" => ProviderType::BitcoinNode,
            _ => ProviderType::BitcoinNode, // Default to Bitcoin node
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
    /// Optional QuickNode endpoint for fallback
    pub quicknode_endpoint: Option<String>,
    /// Provider type to use for this network
    pub provider_type: ProviderType,
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
    /// BTC auto-seeder: proactively pull historical UTXOs via Maestro for
    /// every monitored (charm-holder) address that has not been seeded yet.
    pub btc_auto_seeder_enabled: bool,
    /// How many unseeded addresses the worker pulls per loop iteration.
    pub btc_auto_seeder_batch_size: u64,
    /// Maximum concurrent Maestro requests in-flight.
    pub btc_auto_seeder_concurrency: usize,
    /// Sleep between batches when work exists, milliseconds.
    pub btc_auto_seeder_batch_interval_ms: u64,
    /// Sleep when no unseeded addresses remain, milliseconds.
    pub btc_auto_seeder_idle_interval_ms: u64,
    /// Maestro API key (PRIVATE). Empty disables the seeder.
    pub private_maestro_api_key: String,
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

        // Load Bitcoin testnet4 configuration if enabled.
        // Default OFF: mainnet-only is the supported configuration; testnet4
        // code paths are kept latent but require explicit opt-in.
        let enable_bitcoin_testnet4 = env::var("ENABLE_BITCOIN_TESTNET4")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        if enable_bitcoin_testnet4 {
            let provider_type = ProviderType::parse(
                &env::var("BITCOIN_TESTNET4_PROVIDER")
                    .unwrap_or_else(|_| "bitcoin_node".to_string())
            );
            
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
                    quicknode_endpoint: None, // Testnet4 uses local node only
                    provider_type,
                },
            );
        }

        // Load Bitcoin mainnet configuration if enabled. Default ON: this is
        // the supported configuration for the production indexer.
        let enable_bitcoin_mainnet = env::var("ENABLE_BITCOIN_MAINNET")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        if enable_bitcoin_mainnet {
            let provider_type = ProviderType::parse(
                &env::var("BITCOIN_MAINNET_PROVIDER")
                    .unwrap_or_else(|_| "bitcoin_node".to_string())
            );
            
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
                        .expect("BITCOIN_MAINNET_GENESIS_BLOCK_HEIGHT environment variable is required")
                        .parse::<u64>()
                        .expect("BITCOIN_MAINNET_GENESIS_BLOCK_HEIGHT must be a valid u64"),
                    quicknode_endpoint: env::var("BITCOIN_MAINNET_QUICKNODE_ENDPOINT").ok(),
                    provider_type,
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
            btc_auto_seeder_enabled: env::var("ENABLE_BTC_AUTO_SEEDER")
                .unwrap_or_else(|_| "true".to_string())
                .parse::<bool>()
                .unwrap_or(true),
            btc_auto_seeder_batch_size: env::var("BTC_AUTO_SEEDER_BATCH_SIZE")
                .unwrap_or_else(|_| "10".to_string())
                .parse::<u64>()
                .unwrap_or(10),
            btc_auto_seeder_concurrency: env::var("BTC_AUTO_SEEDER_CONCURRENCY")
                .unwrap_or_else(|_| "5".to_string())
                .parse::<usize>()
                .unwrap_or(5),
            btc_auto_seeder_batch_interval_ms: env::var("BTC_AUTO_SEEDER_BATCH_INTERVAL_MS")
                .unwrap_or_else(|_| "5000".to_string())
                .parse::<u64>()
                .unwrap_or(5000),
            btc_auto_seeder_idle_interval_ms: env::var("BTC_AUTO_SEEDER_IDLE_INTERVAL_MS")
                .unwrap_or_else(|_| "30000".to_string())
                .parse::<u64>()
                .unwrap_or(30000),
            private_maestro_api_key: env::var("PRIVATE_MAESTRO_API_KEY").unwrap_or_default(),
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
