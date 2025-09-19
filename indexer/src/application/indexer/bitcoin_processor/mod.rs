mod batch_processor;
mod block_processor;
mod parallel_tx_processor;
mod retry_handler;
mod summary_updater;

pub use batch_processor::BatchProcessor;
pub use block_processor::BlockProcessor;
pub use parallel_tx_processor::{ParallelTransactionProcessor, ParallelConfig};
pub use retry_handler::RetryHandler;
pub use summary_updater::SummaryUpdater;

use std::time::Duration;
use tokio::time;

use async_trait::async_trait;

use crate::application::indexer::processor_trait::BlockchainProcessor;
use crate::config::{AppConfig, NetworkId};
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::{BitcoinClient, SimpleBitcoinClient, ProviderFactory};
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
    _retry_handler: RetryHandler,
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
            _retry_handler: RetryHandler::new(),
        }
    }

    pub fn network_id(&self) -> &NetworkId {
        self.bitcoin_client.network_id()
    }

    pub async fn initialize_block_height(&mut self) {
        logging::log_info(&format!("[{}] Initializing block height...", self.network_id().name));
        
        match self.bookmark_repository.get_last_processed_block(self.network_id()).await {
            Ok(Some(height)) => {
                self.current_height = height + 1;
                logging::log_info(&format!("[{}] Resuming from block {}", self.network_id().name, self.current_height));
            }
            Ok(None) => {
                logging::log_info(&format!("[{}] No previous bookmark found, starting from genesis block {}", self.network_id().name, self.genesis_block_height));
                self.current_height = self.genesis_block_height;
                logging::log_info(&format!("[{}] Starting from block {}", self.network_id().name, self.current_height));
            }
            Err(e) => {
                logging::log_error(&format!("[{}] Error getting bookmark: {}, starting from genesis block", self.network_id().name, e));
                self.current_height = self.genesis_block_height;
                logging::log_info(&format!("[{}] Starting from block {}", self.network_id().name, self.current_height));
            }
        }
    }

    pub async fn process_available_blocks(&mut self) -> Result<(), BlockProcessorError> {
        logging::log_info(&format!(
            "[{}] 🔍 Starting get_block_count to determine latest height...",
            self.network_id().name
        ));
        
        let latest_height = self.bitcoin_client.get_block_count().await.map_err(|e| {
            logging::log_error(&format!("[{}] ❌ Failed to get block count: {}", self.network_id().name, e));
            BlockProcessorError::BitcoinClientError(e)
        })?;
        
        logging::log_info(&format!(
            "[{}] ✅ Got latest height: {} (current: {})",
            self.network_id().name, latest_height, self.current_height
        ));

        logging::log_info(&format!("[{}] Processing blocks {} to {} (latest)", self.network_id().name, self.current_height, latest_height));

        if self.current_height > latest_height {
            logging::log_info(&format!(
                "[{}] ⏸️ Current height {} is ahead of latest {}, waiting for new blocks...",
                self.network_id().name, self.current_height, latest_height
            ));
            return Ok(());
        }
        
        let total_blocks = latest_height - self.current_height + 1;
        logging::log_info(&format!(
            "[{}] 🚀 Starting to process {} blocks from {} to {}",
            self.network_id().name, total_blocks, self.current_height, latest_height
        ));

        while self.current_height <= latest_height {
            let block_processor = BlockProcessor::new(
                &self.bitcoin_client,
                &self.charm_service,
                &self.bookmark_repository,
                &self.transaction_repository,
                &self.summary_repository,
            );

            // Log processing progress
            let remaining_blocks = latest_height - self.current_height;
            logging::log_info(&format!(
                "[{}] 🔄 Processing block {} (height: {}, {}/{} blocks, {} remaining)",
                self.network_id().name, 
                self.current_height,
                self.current_height,
                self.current_height - (latest_height - total_blocks + 1) + 1,
                total_blocks,
                remaining_blocks
            ));
            
            match block_processor.process_block(self.current_height, self.network_id()).await {
                Ok(()) => {
                    logging::log_info(&format!(
                        "[{}] ✅ Block {} processed successfully, moving to next block",
                        self.network_id().name, self.current_height
                    ));
                    self.current_height += 1;
                }
                Err(BlockProcessorError::BitcoinClientError(ref e)) => {
                    logging::log_error(&format!(
                        "[{}] ❌ Bitcoin client error at block {}: {}",
                        self.network_id().name, self.current_height, e
                    ));
                    
                    // Check if this is a pruned data error
                    let error_msg = e.to_string().to_lowercase();
                    if error_msg.contains("pruned") || 
                       error_msg.contains("block not available") ||
                       error_msg.contains("block height out of range") ||
                       error_msg.contains("block not found") {

                        logging::log_info(&format!("[{}] Block {} appears to be pruned/missing, skipping to next block", self.network_id().name, self.current_height));
                        
                        let next_available = self.current_height + 1;
                        logging::log_info(&format!("[{}] Skipping to block {}", self.network_id().name, next_available));

                        // Update bookmark for skipped block to show progress in API
                        let placeholder_hash = format!("skipped-{}", self.current_height);
                        if let Err(_bookmark_err) = self.bookmark_repository
                            .save_bookmark(&placeholder_hash, self.current_height, false, self.network_id())
                            .await {
                            // Silently continue on bookmark errors
                        }

                        self.current_height = next_available;
                    } else {
                        // For other Bitcoin client errors, propagate them
                        return Err(BlockProcessorError::BitcoinClientError(e.clone()));
                    }
                }
                Err(e) => {
                    logging::log_error(&format!(
                        "[{}] ❌ Non-Bitcoin error at block {}: {}",
                        self.network_id().name, self.current_height, e
                    ));
                    // For non-Bitcoin client errors, propagate them
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl BlockchainProcessor for BitcoinProcessor {
    fn network_id(&self) -> &NetworkId {
        self.bitcoin_client.network_id()
    }

    async fn start_processing(&mut self) -> Result<(), BlockProcessorError> {
        self.initialize_block_height().await;

        loop {
            if let Err(e) = self.process_available_blocks().await {
                logging::log_error(&format!(
                    "[{}] ❌ Error processing blocks: {}.",
                    self.network_id().name,
                    e
                ));
            }

            time::sleep(Duration::from_millis(self.config.indexer.process_interval_ms)).await;
        }
    }

    async fn process_block(&self, height: u64) -> Result<(), BlockProcessorError> {
        let block_processor = BlockProcessor::new(
            &self.bitcoin_client,
            &self.charm_service,
            &self.bookmark_repository,
            &self.transaction_repository,
            &self.summary_repository,
        );

        block_processor.process_block(height, self.network_id()).await
    }
}
