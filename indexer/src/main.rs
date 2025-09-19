use charms_indexer::application::indexer::NetworkManager;
use charms_indexer::config::AppConfig;
use charms_indexer::infrastructure::persistence::{DbPool, RepositoryFactory};
use charms_indexer::utils::logging;

#[tokio::main]
async fn main() {
    logging::init_logger();
    
    // Log version information for deployment tracking
    let _version = env!("CARGO_PKG_VERSION");

    let config = AppConfig::from_env();

    // Log connection details

    // Network configuration logging
    if config.indexer.enable_bitcoin_testnet4 {
        if let Some(_btc_config) = config.get_bitcoin_config("testnet4") {
        }
    }

    if config.indexer.enable_bitcoin_mainnet {
        if let Some(_btc_config) = config.get_bitcoin_config("mainnet") {
        }
    }

    if config.indexer.enable_cardano {
    }

    // Database connection and processor initialization
    match DbPool::new(&config).await {
        Ok(db_pool) => {

            let repositories = RepositoryFactory::create_repositories(&db_pool);

            let mut network_manager = NetworkManager::new(config.clone());

            match network_manager
                .initialize(
                    repositories.charm,
                    repositories.asset,
                    repositories.transaction,
                    repositories.bookmark,
                    repositories.summary,
                )
                .await
            {
                Ok(_) => {
                    // Start processors and handle shutdown

                    if let Err(e) = network_manager.start_all().await {
                        logging::log_error(&format!("Error starting processors: {}", e));
                    }

                    tokio::signal::ctrl_c()
                        .await
                        .expect("Failed to listen for Ctrl+C");
                    network_manager.stop_all().await;
                }
                Err(e) => {
                    logging::log_error(&format!("Error initializing processors: {}", e));
                }
            }
        }
        Err(e) => logging::log_error(&format!("Failed to connect to database: {}", e)),
    }
}
