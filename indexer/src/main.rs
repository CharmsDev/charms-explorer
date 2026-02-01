//! Charms Indexer - Bitcoin blockchain indexer for Charms protocol
//!
//! ## Operation Modes
//!
//! - **Production Mode** (default): Indexes new blocks + mempool in real-time
//! - **Reindex Mode** (`REINDEX_MODE=true`): Batch processes historical blocks, then exits
//!
//! ## Usage
//!
//! ```bash
//! # Production mode (default)
//! cargo run --release
//!
//! # Reindex mode
//! REINDEX_MODE=true cargo run --release
//! ```

use charms_indexer::application::indexer::NetworkManager;
use charms_indexer::application::reindexer;
use charms_indexer::config::AppConfig;
use charms_indexer::infrastructure::persistence::{DbPool, RepositoryFactory};
use charms_indexer::utils::logging;

#[tokio::main]
async fn main() {
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

    // ═══════════════════════════════════════════════════════════════════════
    // MODE SWITCH: Reindex vs Production
    // ═══════════════════════════════════════════════════════════════════════

    if reindexer::is_enabled() {
        // REINDEX MODE: Batch process historical blocks, then exit
        if let Err(e) = reindexer::run(
            &config,
            repositories.charm.clone(),
            repositories.asset.clone(),
            repositories.spell.clone(),
            repositories.stats_holders.clone(),
            repositories.transaction.clone(),
            repositories.bookmark.clone(),
        )
        .await
        {
            logging::log_error(&format!("Reindex failed: {}", e));
        }
        return; // Exit after reindexing
    }

    // PRODUCTION MODE: Real-time indexing of new blocks + mempool
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
            repositories.bookmark.clone(),
            repositories.summary.clone(),
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
