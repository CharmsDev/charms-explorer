//! Charms Indexer - Bitcoin blockchain indexer for Charms protocol
//!
//! Unified indexer that handles both live indexing and reindexing from cached data.
//! Uses block_status table to track downloaded/processed state.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --release
//! ```

use charms_indexer::application::indexer::NetworkManager;
use charms_indexer::config::AppConfig;
use charms_indexer::infrastructure::persistence::{DbPool, RepositoryFactory};
use charms_indexer::utils::logging;

fn main() {
    // Build tokio runtime with 8MB worker thread stack (default 2MB)
    // Required for deep recursion in charms-data parsing
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");

    runtime.block_on(async_main());
}

async fn async_main() {
    logging::init_logger();

    let config = AppConfig::from_env();

    // Connect to database
    let db_pool = match DbPool::new(&config).await {
        Ok(pool) => pool,
        Err(e) => {
            logging::log_error(&format!("Failed to connect to database: {}", e));
            return;
        }
    };

    let repositories = RepositoryFactory::create_repositories(&db_pool);

    // Unified indexer - uses block_status to determine what needs processing
    run_production_indexer(config, repositories).await;
}

/// Production indexer - runs indefinitely processing new blocks
async fn run_production_indexer(
    config: AppConfig,
    repositories: charms_indexer::infrastructure::persistence::Repositories,
) {
    let mut network_manager = NetworkManager::new(config.clone());

    match network_manager
        .initialize(
            repositories.charm.clone(),
            repositories.asset.clone(),
            repositories.spell.clone(),
            repositories.stats_holders.clone(),
            repositories.dex_orders.clone(),
            repositories.transaction.clone(),
            repositories.summary.clone(),
            repositories.block_status.clone(),
            repositories.utxo.clone(),
            repositories.monitored_addresses.clone(),
        )
        .await
    {
        Ok(_) => {
            logging::log_info("═══════════════════════════════════════════════════════════════");
            logging::log_info("  PRODUCTION MODE - Real-time blockchain indexing");
            logging::log_info("═══════════════════════════════════════════════════════════════");

            if let Err(e) = network_manager.start_all().await {
                logging::log_error(&format!("Error starting processors: {}", e));
                return;
            }

            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for Ctrl+C");

            logging::log_info("\nShutting down...");
            network_manager.stop_all().await;
        }
        Err(e) => {
            logging::log_error(&format!("Error initializing processors: {}", e));
        }
    }
}
