//! Reindexer Module - Historical block reprocessing
//!
//! Batch reindexing for rebuilding derived tables (charms, assets, spells)
//! from stored transaction hex data WITHOUT RPC calls.
//!
//! ## Usage
//!
//! ```bash
//! REINDEX_MODE=true cargo run --release
//! ```
//!
//! ## Architecture
//!
//! - **Production Mode** (default): Real-time indexing of new blocks + mempool
//! - **Reindex Mode**: Batch processes historical blocks, then exits
//!
//! ## Module Structure
//!
//! - `types` - Common batch types (SpellBatch, CharmBatch, AssetBatch)
//! - `block_parser` - Transaction parsing and spent detection
//! - `stats_updater` - Stats holders balance updates
//! - `batch_processor` - Main orchestration

mod batch_processor;
mod block_parser;
mod stats_updater;
mod types;

pub use batch_processor::BatchReindexer;

use crate::config::AppConfig;
use crate::domain::errors::BlockProcessorError;
use crate::infrastructure::persistence::repositories::{
    AssetRepository, BookmarkRepository, CharmRepository, SpellRepository, StatsHoldersRepository,
    TransactionRepository,
};
use crate::utils::logging;

/// Check if reindex mode is enabled via environment variable
pub fn is_enabled() -> bool {
    std::env::var("REINDEX_MODE")
        .or_else(|_| std::env::var("FAST_REINDEX_MODE")) // Backward compatibility
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false)
}

/// Run the reindexer for all configured networks
pub async fn run(
    config: &AppConfig,
    charm_repository: CharmRepository,
    asset_repository: AssetRepository,
    spell_repository: SpellRepository,
    stats_holders_repository: StatsHoldersRepository,
    transaction_repository: TransactionRepository,
    bookmark_repository: BookmarkRepository,
) -> Result<(), BlockProcessorError> {
    logging::log_info("═══════════════════════════════════════════════════════════════");
    logging::log_info("  REINDEX MODE - Batch processing historical blocks");
    logging::log_info("═══════════════════════════════════════════════════════════════");

    let networks: Vec<&str> = vec!["mainnet", "testnet4"]
        .into_iter()
        .filter(|n| config.get_bitcoin_config(n).is_some())
        .collect();

    for network in networks {
        logging::log_info(&format!("\n▶ Starting reindex for {}", network));

        if config.get_bitcoin_config(network).is_none() {
            continue;
        }

        let reindexer = BatchReindexer::new(
            network,
            charm_repository.clone(),
            asset_repository.clone(),
            spell_repository.clone(),
            stats_holders_repository.clone(),
            transaction_repository.clone(),
            bookmark_repository.clone(),
        );

        match reindexer.run().await {
            Ok(last_block) => {
                if let Some(height) = last_block {
                    logging::log_info(&format!(
                        "✓ {} reindex complete. Last block: {}",
                        network, height
                    ));
                }
            }
            Err(e) => {
                logging::log_error(&format!("✗ {} reindex failed: {}", network, e));
                return Err(e);
            }
        }
    }

    logging::log_info("\n═══════════════════════════════════════════════════════════════");
    logging::log_info("  REINDEX COMPLETE - Exiting");
    logging::log_info("═══════════════════════════════════════════════════════════════");

    Ok(())
}
