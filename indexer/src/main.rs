use charms_indexer::application::indexer::NetworkManager;
use charms_indexer::config::AppConfig;
use charms_indexer::infrastructure::persistence::{DbPool, RepositoryFactory};
use charms_indexer::utils::logging;

#[tokio::main]
async fn main() {
    logging::init_logger();
    
    // Log version information for deployment tracking
    let version = env!("CARGO_PKG_VERSION");
    logging::log_info(&format!("🚀 Charms Indexer v{} Starting", version));

    let config = AppConfig::from_env();

    // Log connection details
    logging::log_info(&format!("Charms API URL: {}", config.api.url));
    logging::log_database_connection_details(&config.database.url);

    // Network configuration logging
    if config.indexer.enable_bitcoin_testnet4 {
        if let Some(btc_config) = config.get_bitcoin_config("testnet4") {
            logging::log_info("Bitcoin Testnet4 indexing is enabled");
            logging::log_bitcoin_connection_details(
                &btc_config.host,
                &btc_config.port,
                &btc_config.username,
                &btc_config.password,
                "testnet4",
            );
            logging::log_info(&format!(
                "Testnet4 genesis block height: {}",
                btc_config.genesis_block_height
            ));
        }
    }

    if config.indexer.enable_bitcoin_mainnet {
        if let Some(btc_config) = config.get_bitcoin_config("mainnet") {
            logging::log_info("Bitcoin Mainnet indexing is enabled");
            logging::log_bitcoin_connection_details(
                &btc_config.host,
                &btc_config.port,
                &btc_config.username,
                &btc_config.password,
                "mainnet",
            );
            logging::log_info(&format!(
                "Mainnet genesis block height: {}",
                btc_config.genesis_block_height
            ));
        }
    }

    if config.indexer.enable_cardano {
        logging::log_info("Cardano indexing is enabled (not yet implemented)");
    }

    // Database connection and processor initialization
    match DbPool::new(&config).await {
        Ok(db_pool) => {
            logging::log_info(&format!(
                "Successfully connected to database: {}",
                config.database.url
            ));

            let repositories = RepositoryFactory::create_repositories(&db_pool);

            let mut network_manager = NetworkManager::new(config.clone());

            logging::log_info("Initializing blockchain processors");
            match network_manager
                .initialize(
                    repositories.bookmark,
                    repositories.charm,
                    repositories.transaction,
                    repositories.summary,
                )
                .await
            {
                Ok(_) => {
                    // Start processors and handle shutdown
                    logging::log_info("Starting all blockchain processors");

                    if let Err(e) = network_manager.start_all().await {
                        logging::log_error(&format!("Error starting processors: {}", e));
                    }

                    tokio::signal::ctrl_c()
                        .await
                        .expect("Failed to listen for Ctrl+C");
                    logging::log_info("Received shutdown signal, stopping processors...");
                    network_manager.stop_all().await;
                    logging::log_info("All processors stopped, exiting...");
                }
                Err(e) => {
                    logging::log_error(&format!("Error initializing processors: {}", e));
                }
            }
        }
        Err(e) => logging::log_error(&format!("Failed to connect to database: {}", e)),
    }
}
