//! Block processing module — modular architecture.
//!
//! Each sub-module has a single responsibility:
//! - `processor`: slim orchestrator for individual block processing
//! - `detection`: charm detection from transactions using TxAnalyzer
//! - `spent_tracker`: marks charms as spent
//! - `utxo_indexer`: registers addresses and tracks UTXOs
//! - `mempool_consolidator`: promotes mempool entries to confirmed
//! - `reindex`: processes blocks from cached transactions
//! - `batch`: batch persistence for charms, transactions, assets
//! - `summary`: summary statistics updater
//! - `retry`: retry handler with exponential backoff

pub mod batch;
pub mod detection;
pub mod mempool_consolidator;
pub mod processor;
pub mod reindex;
pub mod retry;
pub mod spent_tracker;
pub mod summary;
pub mod utxo_indexer;

pub use batch::{AssetBatchItem, BatchProcessor, CharmBatchItem, TransactionBatchItem};
pub use processor::BlockProcessor;
pub use retry::RetryHandler;
pub use summary::SummaryUpdater;

use std::time::Duration;
use tokio::time;

use async_trait::async_trait;

use crate::application::indexer::processor_trait::BlockchainProcessor;
use crate::config::{AppConfig, NetworkId};
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::BitcoinClient;
use crate::infrastructure::persistence::repositories::{
    BlockStatusRepository, MempoolSpendsRepository, MonitoredAddressesRepository,
    SummaryRepository, TransactionRepository, UtxoRepository,
};
use crate::utils::logging;

/// Top-level processor: handles the block processing loop (live + reindex).
#[derive(Debug)]
pub struct BitcoinProcessor {
    bitcoin_client: BitcoinClient,
    charm_service: CharmService,
    transaction_repository: TransactionRepository,
    summary_repository: SummaryRepository,
    block_status_repository: BlockStatusRepository,
    utxo_repository: UtxoRepository,
    monitored_addresses_repository: MonitoredAddressesRepository,
    mempool_spends_repository: MempoolSpendsRepository,
    config: AppConfig,
    current_height: u64,
    genesis_block_height: u64,
}

impl BitcoinProcessor {
    pub fn new(
        bitcoin_client: BitcoinClient,
        charm_service: CharmService,
        transaction_repository: TransactionRepository,
        summary_repository: SummaryRepository,
        block_status_repository: BlockStatusRepository,
        utxo_repository: UtxoRepository,
        monitored_addresses_repository: MonitoredAddressesRepository,
        mempool_spends_repository: MempoolSpendsRepository,
        config: AppConfig,
        genesis_block_height: u64,
    ) -> Self {
        Self {
            bitcoin_client,
            charm_service,
            transaction_repository,
            summary_repository,
            block_status_repository,
            utxo_repository,
            monitored_addresses_repository,
            mempool_spends_repository,
            current_height: genesis_block_height,
            config,
            genesis_block_height,
        }
    }

    pub fn network_id(&self) -> &NetworkId {
        self.bitcoin_client.network_id()
    }

    fn create_block_processor(&self) -> BlockProcessor {
        BlockProcessor::new(
            self.bitcoin_client.clone(),
            self.charm_service.clone(),
            self.transaction_repository.clone(),
            self.summary_repository.clone(),
            self.block_status_repository.clone(),
            self.utxo_repository.clone(),
            self.monitored_addresses_repository.clone(),
            self.mempool_spends_repository.clone(),
        )
    }

    pub async fn initialize_block_height(&mut self) {
        logging::log_info(&format!(
            "[{}] Initializing block height...",
            self.network_id().name
        ));

        match self
            .block_status_repository
            .get_last_processed_block(self.network_id())
            .await
        {
            Ok(Some(height)) => {
                self.current_height = (height + 1) as u64;
                logging::log_info(&format!(
                    "[{}] Resuming from block {}",
                    self.network_id().name, self.current_height
                ));
            }
            Ok(None) => {
                self.current_height = self.genesis_block_height;
                logging::log_info(&format!(
                    "[{}] Starting from genesis block {}",
                    self.network_id().name, self.current_height
                ));
            }
            Err(e) => {
                logging::log_error(&format!(
                    "[{}] Error getting block_status: {}, starting from genesis",
                    self.network_id().name, e
                ));
                self.current_height = self.genesis_block_height;
            }
        }
    }

    /// Process pending blocks from cache (reindex mode).
    pub async fn process_pending_blocks_from_cache(&self) -> Result<(), BlockProcessorError> {
        let mut total_processed: usize = 0;

        loop {
            let pending_blocks = self
                .block_status_repository
                .get_pending_blocks(self.network_id(), 10000)
                .await
                .map_err(|e| BlockProcessorError::ProcessingError(format!("DB error: {}", e)))?;

            if pending_blocks.is_empty() {
                if total_processed == 0 {
                    logging::log_info(&format!(
                        "[{}] ✅ No pending blocks to reindex",
                        self.network_id().name
                    ));
                } else {
                    logging::log_info(&format!(
                        "[{}] ✅ Reindex complete: {} total blocks processed",
                        self.network_id().name, total_processed
                    ));
                }
                return Ok(());
            }

            let batch_size = pending_blocks.len();
            logging::log_info(&format!(
                "[{}] ♻️ Starting reindex batch of {} pending blocks (total so far: {})",
                self.network_id().name, batch_size, total_processed
            ));

            for (i, height) in pending_blocks.iter().enumerate() {
                let bp = self.create_block_processor();
                if let Err(e) = bp.process_block_from_cache(*height as u64, self.network_id()).await {
                    logging::log_error(&format!(
                        "[{}] ❌ Error reindexing block {}: {}",
                        self.network_id().name, height, e
                    ));
                }

                if (i + 1) % 100 == 0 {
                    logging::log_info(&format!(
                        "[{}] ♻️ Reindex progress: {}/{} blocks, {} total — height: {}",
                        self.network_id().name, i + 1, batch_size, total_processed + i + 1, height
                    ));
                }
            }

            total_processed += batch_size;
            logging::log_info(&format!(
                "[{}] ✅ Batch complete: {} blocks, {} total processed",
                self.network_id().name, batch_size, total_processed
            ));
        }
    }

    pub async fn process_available_blocks(&mut self) -> Result<(), BlockProcessorError> {
        let latest_height = self.bitcoin_client.get_block_count().await.map_err(|e| {
            logging::log_error(&format!(
                "[{}] ❌ Failed to get block count: {}",
                self.network_id().name, e
            ));
            BlockProcessorError::BitcoinClientError(e)
        })?;

        if self.current_height > latest_height {
            static LAST_WAIT_LOG: std::sync::atomic::AtomicU64 =
                std::sync::atomic::AtomicU64::new(0);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let last = LAST_WAIT_LOG.load(std::sync::atomic::Ordering::Relaxed);
            if now - last >= 30 {
                LAST_WAIT_LOG.store(now, std::sync::atomic::Ordering::Relaxed);
                logging::log_info(&format!(
                    "[{}] ⏸️ Waiting for new blocks (current: {}, latest: {})...",
                    self.network_id().name, self.current_height, latest_height
                ));
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            return Ok(());
        }

        while self.current_height <= latest_height {
            let bp = self.create_block_processor();

            match bp.process_block(self.current_height, self.network_id()).await {
                Ok(()) => {
                    self.current_height += 1;
                }
                Err(BlockProcessorError::BitcoinClientError(ref e)) => {
                    let error_msg = e.to_string().to_lowercase();
                    if error_msg.contains("pruned")
                        || error_msg.contains("block not available")
                        || error_msg.contains("block height out of range")
                        || error_msg.contains("block not found")
                    {
                        logging::log_info(&format!(
                            "[{}] Block {} pruned/missing, skipping",
                            self.network_id().name, self.current_height
                        ));

                        let _ = self.block_status_repository
                            .mark_downloaded(self.current_height as i32, None, 0, self.network_id())
                            .await;
                        let _ = self.block_status_repository
                            .mark_processed(self.current_height as i32, 0, self.network_id())
                            .await;
                        self.current_height += 1;
                    } else {
                        return Err(BlockProcessorError::BitcoinClientError(e.clone()));
                    }
                }
                Err(e) => {
                    logging::log_error(&format!(
                        "[{}] ❌ Error at block {}: {}",
                        self.network_id().name, self.current_height, e
                    ));
                    return Err(e);
                }
            }
        }

        self.confirm_pending_blocks(latest_height).await;
        Ok(())
    }

    /// Retroactively confirm blocks with 6+ confirmations
    async fn confirm_pending_blocks(&self, latest_height: u64) {
        match self
            .block_status_repository
            .get_unconfirmed_blocks(self.network_id())
            .await
        {
            Ok(unconfirmed) => {
                let mut confirmed_count = 0;
                for block_height in unconfirmed {
                    let confirmations = latest_height.saturating_sub(block_height as u64) + 1;
                    if confirmations >= 6 {
                        let _ = self
                            .block_status_repository
                            .mark_confirmed(block_height, self.network_id())
                            .await;
                        confirmed_count += 1;
                    }
                }
                if confirmed_count > 0 {
                    logging::log_info(&format!(
                        "[{}] ✅ Confirmed {} previously unconfirmed blocks",
                        self.network_id().name, confirmed_count
                    ));
                }
            }
            Err(e) => {
                logging::log_warning(&format!(
                    "[{}] ⚠️ Failed to check unconfirmed blocks: {}",
                    self.network_id().name, e
                ));
            }
        }
    }
}

#[async_trait]
impl BlockchainProcessor for BitcoinProcessor {
    fn network_id(&self) -> &NetworkId {
        self.bitcoin_client.network_id()
    }

    async fn start_processing(&mut self) -> Result<(), BlockProcessorError> {
        self.process_pending_blocks_from_cache().await?;
        self.initialize_block_height().await;

        loop {
            if let Err(e) = self.process_available_blocks().await {
                logging::log_error(&format!(
                    "[{}] ❌ Error processing blocks: {}.",
                    self.network_id().name, e
                ));
            }

            time::sleep(Duration::from_millis(
                self.config.indexer.process_interval_ms,
            ))
            .await;
        }
    }

    async fn process_block(&self, height: u64) -> Result<(), BlockProcessorError> {
        let bp = self.create_block_processor();
        bp.process_block(height, self.network_id()).await
    }
}
