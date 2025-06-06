use std::thread;
use std::time::Duration;

use serde_json::json;

use crate::config::AppConfig;
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::BitcoinClient;
use crate::infrastructure::persistence::repositories::{BookmarkRepository, TransactionRepository};
use crate::utils::logging;

/// Block processor for finding charm transactions in Bitcoin blocks
pub struct BlockProcessor {
    bitcoin_client: BitcoinClient,
    charm_service: CharmService,
    bookmark_repository: BookmarkRepository,
    transaction_repository: TransactionRepository,
    config: AppConfig,
    current_height: u64,
}

impl BlockProcessor {
    pub fn new(
        bitcoin_client: BitcoinClient,
        charm_service: CharmService,
        bookmark_repository: BookmarkRepository,
        transaction_repository: TransactionRepository,
        config: AppConfig,
    ) -> Self {
        Self {
            bitcoin_client,
            charm_service,
            bookmark_repository,
            transaction_repository,
            current_height: config.indexer.genesis_block_height,
            config,
        }
    }

    /// Start processing blocks
    pub async fn start_processing(&mut self) -> Result<(), BlockProcessorError> {
        // Get the last processed block from the database
        match self.bookmark_repository.get_last_processed_block().await {
            Ok(Some(height)) => {
                // Start from the next block after the last processed one
                self.current_height = height + 1;
                logging::log_info(&format!(
                    "Resuming from block height: {}",
                    self.current_height
                ));
            }
            Ok(None) => {
                // No blocks processed yet, start from genesis
                logging::log_info(&format!(
                    "Starting from genesis block height: {}",
                    self.current_height
                ));
            }
            Err(e) => {
                logging::log_error(&format!(
                    "Database error getting last processed block: {}",
                    e
                ));
                return Err(BlockProcessorError::DbError(e));
            }
        }

        loop {
            // Get the latest block height
            match self.bitcoin_client.get_block_count() {
                Ok(latest_height) => {
                    // Process new blocks if available
                    while self.current_height <= latest_height {
                        self.process_block(self.current_height).await?;
                        self.current_height += 1;
                    }

                    // If we've processed all available blocks, wait for new ones
                    if self.current_height > latest_height {
                        logging::log_info(&format!(
                            "Waiting for new blocks... Current height: {}",
                            latest_height
                        ));
                        thread::sleep(Duration::from_millis(
                            self.config.indexer.process_interval_ms,
                        ));
                    }
                }
                Err(e) => {
                    logging::log_error(&format!("Error getting block count: {}", e));
                    thread::sleep(Duration::from_millis(
                        self.config.indexer.process_interval_ms,
                    ));
                }
            }
        }
    }

    /// Process a single block
    async fn process_block(&self, height: u64) -> Result<(), BlockProcessorError> {
        logging::log_info(&format!("Processing block: {}", height));

        // Get block hash
        let block_hash = self.bitcoin_client.get_block_hash(height)?;

        // Get block
        let block = self.bitcoin_client.get_block(&block_hash)?;

        // Save bookmark for this block
        // Assume blocks are confirmed after 6 confirmations
        let latest_height = self.bitcoin_client.get_block_count()?;
        let confirmations = latest_height - height + 1;
        let is_confirmed = confirmations >= 6;

        logging::log_info(&format!(
            "Saving bookmark for block {} (hash: {}, confirmed: {})",
            height, block_hash, is_confirmed
        ));

        match self
            .bookmark_repository
            .save_bookmark(&block_hash.to_string(), height, is_confirmed)
            .await
        {
            Ok(_) => {
                logging::log_info(&format!("Successfully saved bookmark for block {}", height));
            }
            Err(e) => {
                logging::log_error(&format!(
                    "Error saving bookmark for block {}: {}",
                    height, e
                ));
                return Err(BlockProcessorError::DbError(e));
            }
        }

        // Collect transactions for batch processing
        let mut charm_batch = Vec::new();
        let mut transaction_batch = Vec::new();

        // Process transactions
        for (tx_pos, tx) in block.txdata.iter().enumerate() {
            let txid = tx.txid();
            let txid_str = txid.to_string();

            // Get raw transaction
            match self.bitcoin_client.get_raw_transaction_hex(&txid_str) {
                Ok(raw_tx_hex) => {
                    // Try to detect and process a charm
                    match self
                        .charm_service
                        .detect_and_process_charm(&txid_str, height)
                        .await
                    {
                        Ok(Some(charm)) => {
                            logging::log_info(&format!(
                                "Block {}: Found charm tx: {} at pos {}",
                                height, txid, tx_pos
                            ));

                            // Store the raw transaction data
                            let raw_json = json!({
                                "hex": raw_tx_hex,
                                "txid": txid_str,
                            });

                            // Add to transaction batch with confirmations and status
                            transaction_batch.push((
                                txid_str.clone(),
                                height,
                                tx_pos as i64,
                                raw_json,
                                charm.data.clone(),
                                confirmations as i32,
                                is_confirmed,
                            ));

                            // Add to charm batch
                            charm_batch.push((
                                txid_str,
                                charm.charmid,
                                height,
                                charm.data,
                                charm.asset_type,
                            ));
                        }
                        Ok(None) => {
                            // Not a charm, skip
                        }
                        Err(e) => {
                            logging::log_error(&format!("Error processing potential charm: {}", e));
                        }
                    }
                }
                Err(e) => {
                    logging::log_error(&format!("Error getting raw transaction: {}", e));
                }
            }
        }

        // Save transactions in batch if any were found
        if !transaction_batch.is_empty() {
            logging::log_info(&format!(
                "Saving batch of {} transactions for block {}",
                transaction_batch.len(),
                height
            ));

            match self
                .transaction_repository
                .save_batch(transaction_batch)
                .await
            {
                Ok(_) => {
                    logging::log_info(&format!(
                        "Successfully saved transaction batch for block {}",
                        height
                    ));
                }
                Err(e) => {
                    logging::log_error(&format!(
                        "Error saving transaction batch for block {}: {}",
                        height, e
                    ));
                    return Err(BlockProcessorError::DbError(e));
                }
            }
        }

        Ok(())
    }
}
