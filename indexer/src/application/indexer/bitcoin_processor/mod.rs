mod batch_processor;
mod block_finder;
mod block_processor;
mod retry_handler;
mod summary_updater;

pub use batch_processor::BatchProcessor;
pub use block_finder::BlockFinder;
pub use block_processor::BlockProcessor;
pub use retry_handler::RetryHandler;
pub use summary_updater::SummaryUpdater;

use std::time::Duration;
use tokio::time;

use async_trait::async_trait;

use crate::application::indexer::processor_trait::BlockchainProcessor;
use crate::config::{AppConfig, NetworkId};
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::BitcoinClient;
use crate::infrastructure::persistence::repositories::{BookmarkRepository, SummaryRepository, TransactionRepository};
use crate::utils::logging;

/// Processes Bitcoin blocks to find and index charm transactions
#[derive(Debug)]
pub struct BitcoinProcessor {
    bitcoin_client: BitcoinClient,
    charm_service: CharmService,
    bookmark_repository: BookmarkRepository,
    transaction_repository: TransactionRepository,
    summary_repository: SummaryRepository,
    config: AppConfig,
    current_height: u64,
    genesis_block_height: u64,
    retry_handler: RetryHandler,
}

impl BitcoinProcessor {
    pub fn new(
        bitcoin_client: BitcoinClient,
        charm_service: CharmService,
        bookmark_repository: BookmarkRepository,
        transaction_repository: TransactionRepository,
        summary_repository: SummaryRepository,
        config: AppConfig,
        genesis_block_height: u64,
    ) -> Self {
        Self {
            bitcoin_client,
            charm_service,
            bookmark_repository,
            transaction_repository,
            summary_repository,
            current_height: genesis_block_height,
            config,
            genesis_block_height,
            retry_handler: RetryHandler::new(),
        }
    }

    /// Returns the network identifier for this processor
    pub fn network_id(&self) -> &NetworkId {
        self.bitcoin_client.network_id()
    }

    /// Sets initial block height from database or genesis
    pub async fn initialize_block_height(&mut self) {
        logging::log_info(&format!(
            "[{}] 🔍 Initializing block height from database...",
            self.network_id().name
        ));

        match self
            .retry_handler
            .execute_with_retry(|| async {
                logging::log_info(&format!(
                    "[{}] 📊 Querying database for last processed block...",
                    self.network_id().name
                ));
                self.bookmark_repository
                    .get_last_processed_block(self.network_id())
                    .await
            })
            .await
        {
            Ok(Some(height)) => {
                self.current_height = height + 1;
                logging::log_info(&format!(
                    "[{}] ✅ Found last processed block: {}, resuming from: {}",
                    self.network_id().name,
                    height,
                    self.current_height
                ));
            }
            Ok(None) => {
                logging::log_info(&format!(
                    "[{}] 📭 No previous blocks found in database. Searching for first available block from genesis height: {}",
                    self.network_id().name,
                    self.genesis_block_height
                ));

                let block_finder = BlockFinder::new(&self.bitcoin_client);
                let first_available = block_finder
                    .find_first_available_block(self.genesis_block_height)
                    .await;
                self.current_height = first_available;

                logging::log_info(&format!(
                    "[{}] 🎯 Will start processing from first available block: {}",
                    self.network_id().name,
                    self.current_height
                ));
            }
            Err(e) => {
                logging::log_error(&format!(
                    "[{}] ❌ Error querying database for last processed block: {}. Starting from genesis.",
                    self.network_id().name,
                    e
                ));

                logging::log_info(&format!(
                    "[{}] 🔄 Searching for first available block from genesis height: {}",
                    self.network_id().name,
                    self.genesis_block_height
                ));

                let block_finder = BlockFinder::new(&self.bitcoin_client);
                let first_available = block_finder
                    .find_first_available_block(self.genesis_block_height)
                    .await;
                self.current_height = first_available;

                logging::log_info(&format!(
                    "[{}] 🎯 Will start processing from first available block: {}",
                    self.network_id().name,
                    self.current_height
                ));
            }
        }
    }

    /// Processes all blocks from current height to chain tip
    pub async fn process_available_blocks(&mut self) -> Result<(), BlockProcessorError> {
        logging::log_info(&format!(
            "[{}] 🔍 Getting current chain height...",
            self.network_id().name
        ));

        let latest_height = self
            .bitcoin_client
            .get_block_count()
            .map_err(BlockProcessorError::BitcoinClientError)?;

        logging::log_info(&format!(
            "[{}] 📊 Current chain height: {}, Processing from: {}",
            self.network_id().name,
            latest_height,
            self.current_height
        ));

        while self.current_height <= latest_height {
            let block_processor = BlockProcessor::new(
                &self.bitcoin_client,
                &self.charm_service,
                &self.bookmark_repository,
                &self.transaction_repository,
                &self.summary_repository,
            );

            match block_processor
                .process_block(self.current_height, self.network_id())
                .await
            {
                Ok(()) => {
                    self.current_height += 1;
                }
                Err(BlockProcessorError::BitcoinClientError(ref e)) => {
                    // Check if this is a pruned data error
                    let error_msg = e.to_string().to_lowercase();
                    if error_msg.contains("pruned") || 
                       error_msg.contains("block not available") ||
                       error_msg.contains("block height out of range") ||
                       error_msg.contains("block not found") {
                        logging::log_info(&format!(
                            "[{}] 🔍 Block {} not available (pruned/missing). Error: {}. Searching for next available block...",
                            self.network_id().name,
                            self.current_height,
                            error_msg
                        ));

                        let block_finder = BlockFinder::new(&self.bitcoin_client);
                        let next_available = block_finder
                            .find_first_available_block(self.current_height + 1)
                            .await;

                        let skipped_blocks = next_available - self.current_height;
                        logging::log_info(&format!(
                            "[{}] 🎯 Skipping to next available block: {} (skipped {} blocks)",
                            self.network_id().name,
                            next_available,
                            skipped_blocks
                        ));

                        // Update bookmark for all skipped blocks to show progress in API
                        if skipped_blocks > 0 {
                            logging::log_info(&format!(
                                "[{}] 💾 Updating bookmarks for skipped blocks {} to {}",
                                self.network_id().name,
                                self.current_height,
                                next_available - 1
                            ));

                            for skip_height in self.current_height..next_available {
                                // Create a placeholder hash for skipped blocks
                                let placeholder_hash = format!("skipped-{}", skip_height);
                                
                                if let Err(bookmark_err) = self.bookmark_repository
                                    .save_bookmark(&placeholder_hash, skip_height, false, self.network_id())
                                    .await {
                                    logging::log_error(&format!(
                                        "[{}] ❌ Error saving bookmark for skipped block {}: {}",
                                        self.network_id().name,
                                        skip_height,
                                        bookmark_err
                                    ));
                                } else {
                                    logging::log_info(&format!(
                                        "[{}] ✅ Saved bookmark for skipped block {}",
                                        self.network_id().name,
                                        skip_height
                                    ));
                                }
                            }
                        }

                        self.current_height = next_available;
                    } else {
                        // For other Bitcoin client errors, propagate them
                        return Err(BlockProcessorError::BitcoinClientError(e.clone()));
                    }
                }
                Err(e) => {
                    // For non-Bitcoin client errors, propagate them
                    return Err(e);
                }
            }
        }

        if self.current_height > latest_height {
            logging::log_info(&format!(
                "[{}] ⏳ Up to date. Waiting for new blocks... Current height: {}",
                self.network_id().name,
                latest_height
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl BlockchainProcessor for BitcoinProcessor {
    fn network_id(&self) -> &NetworkId {
        self.bitcoin_client.network_id()
    }

    /// Starts continuous block processing loop
    async fn start_processing(&mut self) -> Result<(), BlockProcessorError> {
        logging::log_info(&format!(
            "[{}] 🚀 Starting processing for network: {}",
            self.network_id().name,
            self.network_id().to_string()
        ));

        logging::log_info(&format!(
            "[{}] � Initializing block height...",
            self.network_id().name
        ));

        self.initialize_block_height().await;

        logging::log_info(&format!(
            "[{}] � Starting main processing loop...",
            self.network_id().name
        ));

        loop {
            if let Err(e) = self.process_available_blocks().await {
                logging::log_error(&format!(
                    "[{}] ❌ Error processing blocks: {}. Will retry after interval.",
                    self.network_id().name,
                    e
                ));
            }

            logging::log_info(&format!(
                "[{}] 🔒 PROCESSING LOCK: Current height: {}, Sleeping for {} ms before next iteration...",
                self.network_id().name,
                self.current_height,
                self.config.indexer.process_interval_ms
            ));

            time::sleep(Duration::from_millis(
                self.config.indexer.process_interval_ms,
            ))
            .await;
        }
    }

    /// Processes a single block at specified height
    async fn process_block(&self, height: u64) -> Result<(), BlockProcessorError> {
        let block_processor = BlockProcessor::new(
            &self.bitcoin_client,
            &self.charm_service,
            &self.bookmark_repository,
            &self.transaction_repository,
            &self.summary_repository,
        );

        block_processor
            .process_block(height, self.network_id())
            .await
    }
}
