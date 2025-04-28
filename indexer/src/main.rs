use charms_indexer::application::indexer::BlockProcessor;
use charms_indexer::config::AppConfig;
use charms_indexer::domain::services::CharmService;
use charms_indexer::infrastructure::api::ApiClient;
use charms_indexer::infrastructure::bitcoin::BitcoinClient;
use charms_indexer::infrastructure::persistence::{DbPool, RepositoryFactory};
use charms_indexer::utils::logging;
use migration::{Migrator, MigratorTrait};
use sea_orm::Database;

#[tokio::main]
async fn main() {
    // Initialize logger
    logging::init_logger();
    logging::log_info("Bitcoin Testnet 4 Block Parser");
    logging::log_info("Waiting for new blocks and looking for charm transactions...");

    // Load configuration from environment variables
    let config = AppConfig::from_env();

    logging::log_info(&format!(
        "Connecting to Bitcoin node at {}:{}",
        config.bitcoin.host, config.bitcoin.port
    ));
    logging::log_info(&format!("Charms API URL: {}", config.api.url));
    logging::log_info(&format!("Database URL: {}", config.database.url));

    // Run migrations
    logging::log_info("Running database migrations...");
    match Database::connect(&config.database.url).await {
        Ok(connection) => {
            if let Err(e) = Migrator::up(&connection, None).await {
                logging::log_error(&format!("Error running migrations: {}", e));
                return;
            }
            logging::log_info("Migrations completed successfully!");
        }
        Err(e) => {
            logging::log_error(&format!(
                "Failed to connect to database for migrations: {}",
                e
            ));
            return;
        }
    }

    // Create Bitcoin RPC client
    match BitcoinClient::new(&config) {
        Ok(bitcoin_client) => {
            logging::log_info("Successfully connected to Bitcoin node");

            // Create database pool
            match DbPool::new(&config).await {
                Ok(db_pool) => {
                    logging::log_info("Successfully connected to database");

                    // Create repositories
                    let repositories = RepositoryFactory::create_repositories(&db_pool);

                    // Create API client
                    match ApiClient::new(&config) {
                        Ok(api_client) => {
                            // Create charm service
                            let charm_service = CharmService::new(
                                bitcoin_client.clone(),
                                api_client,
                                repositories.charm.clone(),
                            );

                            // Get current block count
                            match bitcoin_client.get_block_count() {
                                Ok(initial_height) => {
                                    logging::log_info(&format!(
                                        "Current block height: {}",
                                        initial_height
                                    ));

                                    // Create block processor
                                    let mut processor = BlockProcessor::new(
                                        bitcoin_client,
                                        charm_service,
                                        repositories.bookmark,
                                        repositories.transaction,
                                        config,
                                    );

                                    // Start processing blocks
                                    logging::log_info("Starting block processor...");
                                    logging::log_info("This will index Bitcoin blocks and look for charm transactions");
                                    logging::log_info(
                                        "Blocks will be marked as confirmed after 6 confirmations",
                                    );
                                    logging::log_info("Transactions will be tracked with their confirmation status");

                                    if let Err(e) = processor.start_processing().await {
                                        logging::log_error(&format!(
                                            "Error processing blocks: {}",
                                            e
                                        ));
                                    }
                                }
                                Err(e) => {
                                    logging::log_error(&format!("Error getting block count: {}", e))
                                }
                            }
                        }
                        Err(e) => {
                            logging::log_error(&format!("Failed to create API client: {}", e))
                        }
                    }
                }
                Err(e) => logging::log_error(&format!("Failed to connect to database: {}", e)),
            }
        }
        Err(e) => logging::log_error(&format!("Failed to connect to Bitcoin node: {}", e)),
    }
}
