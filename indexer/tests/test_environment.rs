use charms_indexer::config::{AppConfig, NetworkId, NetworkType};
use charms_indexer::infrastructure::bitcoin::BitcoinClient;
use charms_indexer::infrastructure::persistence::{DbPool, RepositoryFactory};
use charms_indexer::utils::logging;
use std::env;

#[tokio::main]
async fn main() {
    // Initialize logger
    logging::init_logger();
    logging::log_info("Bitcoin Testnet 4 Parser - Environment Test");

    // Load configuration from environment variables
    let config = AppConfig::from_env();
    logging::log_info("Loaded environment variables");

    // Get Bitcoin testnet4 config
    if let Some(bitcoin_config) = config.get_bitcoin_config("testnet4") {
        logging::log_info(&format!(
            "BITCOIN_TESTNET4_RPC_HOST: {}",
            bitcoin_config.host
        ));
        logging::log_info(&format!(
            "BITCOIN_TESTNET4_RPC_PORT: {}",
            bitcoin_config.port
        ));
        logging::log_info(&format!(
            "BITCOIN_TESTNET4_RPC_USER: {}",
            bitcoin_config.username
        ));
        logging::log_info(&format!(
            "BITCOIN_TESTNET4_RPC_PASSWORD: {}",
            bitcoin_config.password
        ));
        logging::log_info(&format!(
            "BITCOIN_TESTNET4_GENESIS_BLOCK_HEIGHT: {}",
            bitcoin_config.genesis_block_height
        ));
    } else {
        logging::log_error("Bitcoin testnet4 configuration not found");
    }

    // Get Bitcoin mainnet config if enabled
    if let Some(bitcoin_config) = config.get_bitcoin_config("mainnet") {
        logging::log_info(&format!(
            "BITCOIN_MAINNET_RPC_HOST: {}",
            bitcoin_config.host
        ));
        logging::log_info(&format!(
            "BITCOIN_MAINNET_RPC_PORT: {}",
            bitcoin_config.port
        ));
        logging::log_info(&format!(
            "BITCOIN_MAINNET_RPC_USER: {}",
            bitcoin_config.username
        ));
        logging::log_info(&format!(
            "BITCOIN_MAINNET_RPC_PASSWORD: {}",
            bitcoin_config.password
        ));
        logging::log_info(&format!(
            "BITCOIN_MAINNET_GENESIS_BLOCK_HEIGHT: {}",
            bitcoin_config.genesis_block_height
        ));
    }

    logging::log_info(&format!("DATABASE_URL: {}", config.database.url));

    // Test connection to Bitcoin testnet4 node
    logging::log_info("Attempting to connect to Bitcoin testnet4 node...");
    let testnet4_id = NetworkId::new(NetworkType::Bitcoin, "testnet4");
    test_bitcoin_connection(&config, "testnet4", &testnet4_id).await;

    // Test connection to Bitcoin mainnet node
    logging::log_info("Attempting to connect to Bitcoin mainnet node...");
    let mainnet_id = NetworkId::new(NetworkType::Bitcoin, "mainnet");
    test_bitcoin_connection(&config, "mainnet", &mainnet_id).await;
}

/// Test connection to a Bitcoin node
async fn test_bitcoin_connection(config: &AppConfig, network: &str, network_id: &NetworkId) {
    // Get Bitcoin config
    match config.get_bitcoin_config(network) {
        Some(bitcoin_config) => match BitcoinClient::new(bitcoin_config) {
            Ok(client) => {
                logging::log_info(&format!(
                    "Successfully connected to Bitcoin {} node",
                    network
                ));

                match client.get_block_count() {
                    Ok(count) => {
                        logging::log_info(&format!("Current block height: {}", count));
                    }
                    Err(e) => logging::log_error(&format!("Failed to get block count: {}", e)),
                }

                logging::log_info("Attempting to get genesis block hash...");
                match client.get_block_hash(bitcoin_config.genesis_block_height) {
                    Ok(hash) => {
                        logging::log_info(&format!(
                            "Block hash for height {}: {}",
                            bitcoin_config.genesis_block_height, hash
                        ));

                        match client.get_block(&hash) {
                            Ok(block) => logging::log_info(&format!(
                                "Successfully got block with {} transactions",
                                block.txdata.len()
                            )),
                            Err(e) => logging::log_error(&format!("Failed to get block: {}", e)),
                        }
                    }
                    Err(e) => logging::log_error(&format!("Failed to get block hash: {}", e)),
                }
            }
            Err(e) => logging::log_error(&format!(
                "Failed to connect to Bitcoin {} node: {}",
                network, e
            )),
        },
        None => {
            logging::log_error(&format!("Bitcoin {} configuration not found", network));
        }
    }

    logging::log_info("Attempting to connect to database...");
    match DbPool::new(&config).await {
        Ok(db_pool) => {
            logging::log_info("Successfully connected to database");

            let repositories = RepositoryFactory::create_repositories(&db_pool);

            match repositories
                .bookmark
                .get_last_processed_block(&network_id)
                .await
            {
                Ok(Some(height)) => {
                    logging::log_info(&format!("Last processed block height: {}", height));

                    // We don't have a get_all() method, so we'll just print the last processed block
                    logging::log_info(&format!("Last processed block height: {}", height));
                }
                Ok(None) => {
                    logging::log_info("No blocks processed yet");
                }
                Err(e) => logging::log_error(&format!("Failed to get last processed block: {}", e)),
            }

            match repositories
                .bookmark
                .get_last_processed_block(&network_id)
                .await
            {
                Ok(Some(height)) => {
                    logging::log_info(&format!("Last processed block height: {}", height));

                    let page = 0;
                    let page_size = 10;
                    match repositories.charm.find_paginated(page, page_size).await {
                        Ok((charms, total)) => {
                            if !charms.is_empty() {
                                logging::log_info(&format!(
                                    "Found {} charms (showing page {} of {})",
                                    total,
                                    page + 1,
                                    (total as f64 / page_size as f64).ceil() as u64
                                ));

                                for charm in charms.iter().take(5) {
                                    logging::log_info(&format!(
                                        "  Charm ID: {}, Asset Type: {}",
                                        charm.charmid, charm.asset_type
                                    ));
                                }

                                if charms.len() > 5 {
                                    logging::log_info(&format!(
                                        "  ... and {} more",
                                        charms.len() - 5
                                    ));
                                }
                            } else {
                                logging::log_info("No charms found in database");
                            }
                        }
                        Err(e) => logging::log_error(&format!("Failed to get charms: {}", e)),
                    }
                }
                Ok(None) => {
                    logging::log_info("No blocks processed yet");
                }
                Err(e) => logging::log_error(&format!("Failed to get last processed block: {}", e)),
            }
        }
        Err(e) => logging::log_error(&format!("Failed to connect to database: {}", e)),
    }
}
