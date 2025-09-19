//! Block processor for handling individual block processing operations

use bitcoincore_rpc::bitcoin;
use serde_json::json;
use std::sync::Arc;

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::{BitcoinClient, SimpleBitcoinClient};
use crate::infrastructure::persistence::repositories::{BookmarkRepository, SummaryRepository, TransactionRepository};
use crate::utils::logging;

use super::batch_processor::{AssetBatchItem, BatchProcessor, CharmBatchItem, TransactionBatchItem};
use super::parallel_tx_processor::{ParallelTransactionProcessor, ParallelConfig};
use super::retry_handler::RetryHandler;

/// Handles processing of individual blocks
#[derive(Debug)]
pub struct BlockProcessor<'a> {
    bitcoin_client: &'a BitcoinClient,
    charm_service: &'a CharmService,
    bookmark_repository: &'a BookmarkRepository,
    transaction_repository: &'a TransactionRepository,
    summary_repository: &'a SummaryRepository,
    retry_handler: RetryHandler,
}

impl<'a> BlockProcessor<'a> {
    pub fn new(
        bitcoin_client: &'a BitcoinClient,
        charm_service: &'a CharmService,
        bookmark_repository: &'a BookmarkRepository,
        transaction_repository: &'a TransactionRepository,
        summary_repository: &'a SummaryRepository,
    ) -> Self {
        Self {
            bitcoin_client,
            charm_service,
            bookmark_repository,
            transaction_repository,
            summary_repository,
            retry_handler: RetryHandler::new(),
        }
    }

    /// Process a single block
    pub async fn process_block(
        &self,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {

        // Get latest height once for the entire block processing
        let latest_height = self.bitcoin_client.get_block_count().await.map_err(|e| {
            logging::log_error(&format!(
                "[{}] Error getting block count: {}",
                network_id.name, e
            ));
            BlockProcessorError::BitcoinClientError(e)
        })?;

        let block_hash = self.get_block_hash(height, network_id).await?;
        let block = self.get_block(&block_hash, network_id).await?;

        self.save_bookmark(&block_hash, height, latest_height, network_id).await?;

        let (transaction_batch, charm_batch, asset_batch) = self
            .process_transactions(&block, &block_hash, height, latest_height, network_id)
            .await?;

        let batch_processor = BatchProcessor::new(self.charm_service, self.transaction_repository);

        batch_processor
            .save_transaction_batch(transaction_batch.clone(), height, network_id)
            .await?;

        batch_processor
            .save_charm_batch(charm_batch.clone(), height, network_id)
            .await?;

        batch_processor
            .save_asset_batch(asset_batch.clone(), height, network_id)
            .await?;

        // Update summary table with current statistics
        let summary_updater = super::SummaryUpdater::new(self.bitcoin_client, self.summary_repository);
        summary_updater.update_statistics(height, latest_height, &charm_batch, &transaction_batch, network_id)
            .await?;

        // Single summary line per block
        println!("[{}] Block {}: {} txs, {} charms, {} assets", 
            network_id.name, height, transaction_batch.len(), charm_batch.len(), asset_batch.len());

        Ok(())
    }

    /// Get block hash for given height
    async fn get_block_hash(
        &self,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<bitcoin::BlockHash, BlockProcessorError> {
        
        match self.bitcoin_client.get_block_hash(height).await {
            Ok(hash) => {
                Ok(hash)
            }
            Err(e) => {
                logging::log_error(&format!(
                    "[{}] ❌ Error getting block hash for height {}: {}",
                    network_id.name, height, e
                ));
                Err(BlockProcessorError::BitcoinClientError(e))
            }
        }
    }

    /// Get block data for given hash
    async fn get_block(
        &self,
        block_hash: &bitcoin::BlockHash,
        network_id: &NetworkId,
    ) -> Result<bitcoin::Block, BlockProcessorError> {
        
        match self.bitcoin_client.get_block(block_hash).await {
            Ok(block) => {
                Ok(block)
            }
            Err(e) => {
                // Check if this is a pruned block error
                let error_msg = e.to_string();
                if error_msg.contains("pruned data") || error_msg.contains("Block not available") {
                    logging::log_error(&format!(
                        "[{}] ❌ Block {} is pruned/not available: {}",
                        network_id.name, block_hash, e
                    ));
                } else {
                    logging::log_error(&format!(
                        "[{}] ❌ Error getting block for hash {}: {}",
                        network_id.name, block_hash, e
                    ));
                }
                Err(BlockProcessorError::BitcoinClientError(e))
            }
        }
    }

    /// Save bookmark for processed block
    async fn save_bookmark(
        &self,
        block_hash: &bitcoin::BlockHash,
        height: u64,
        latest_height: u64,
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {
        let confirmations = latest_height - height + 1;
        let is_confirmed = confirmations >= 6;


        self.retry_handler
            .execute_with_retry_and_logging(
                || async {
                    self.bookmark_repository
                        .save_bookmark(&block_hash.to_string(), height, is_confirmed, network_id)
                        .await
                },
                "save bookmark",
                &network_id.name,
            )
            .await
            .map_err(BlockProcessorError::DbError)?;

        Ok(())
    }

    /// Process all transactions in a block using parallel processing
    async fn process_transactions(
        &self,
        block: &bitcoin::Block,
        block_hash: &bitcoin::BlockHash,
        height: u64,
        latest_height: u64,
        network_id: &NetworkId,
    ) -> Result<(Vec<TransactionBatchItem>, Vec<CharmBatchItem>, Vec<AssetBatchItem>), BlockProcessorError> {
        let blockchain = "Bitcoin".to_string();
        let network = network_id.name.clone();
        let tx_count = block.txdata.len();

        // Use parallel processing for blocks with many transactions
        if tx_count > 10 {

            // Use optimized configuration based on provider type
            let config = self.get_parallel_config_for_provider(tx_count);
            
            let _provider_name = self.bitcoin_client.get_primary_provider_name()
                .unwrap_or_else(|| "Unknown".to_string());
            let _is_local = self.is_using_local_node();
            

            let parallel_processor = ParallelTransactionProcessor::new(
                Arc::new(self.bitcoin_client.clone()),
                Arc::new(self.charm_service.clone()),
                network_id.clone(),
                Some(config),
            );

            parallel_processor
                .process_block_transactions(
                    block,
                    block_hash,
                    height,
                    latest_height,
                    &blockchain,
                    &network,
                )
                .await
        } else {
            // Use sequential processing for small blocks

            let mut transaction_batch = Vec::new();
            let mut charm_batch = Vec::new();
            let mut asset_batch = Vec::new();

            for (tx_pos, tx) in block.txdata.iter().enumerate() {
                let txid = tx.txid();
                let txid_str = txid.to_string();

                if let Some((transaction_item, charm_item, asset_item)) = self
                    .process_single_transaction(
                        &txid_str,
                        block_hash,
                        height,
                        latest_height,
                        tx_pos,
                        &blockchain,
                        &network,
                        network_id,
                    )
                    .await?
                {
                    transaction_batch.push(transaction_item);
                    charm_batch.push(charm_item);
                    if let Some(asset) = asset_item {
                        asset_batch.push(asset);
                    }
                }
            }

            Ok((transaction_batch, charm_batch, asset_batch))
        }
    }

    /// Process a single transaction
    async fn process_single_transaction(
        &self,
        txid: &str,
        block_hash: &bitcoin::BlockHash,
        height: u64,
        latest_height: u64,
        tx_pos: usize,
        blockchain: &str,
        network: &str,
        network_id: &NetworkId,
    ) -> Result<Option<(TransactionBatchItem, CharmBatchItem, Option<AssetBatchItem>)>, BlockProcessorError> {
        let raw_tx_hex = match self
            .bitcoin_client
            .get_raw_transaction_hex(txid, Some(block_hash))
            .await
        {
            Ok(hex) => hex,
            Err(e) => {
                logging::log_error(&format!(
                    "[{}] ❌ Error getting raw transaction {}: {}",
                    network_id.name, txid, e
                ));
                return Ok(None);
            }
        };

        match self
            .charm_service
            .detect_and_process_charm_with_context(txid, height, Some(block_hash), tx_pos)
            .await
        {
            Ok(Some(charm)) => {

                let confirmations = latest_height - height + 1;
                let is_confirmed = confirmations >= 6;

                let raw_json = json!({
                    "hex": raw_tx_hex,
                    "txid": txid,
                });

                let transaction_item = (
                    txid.to_string(),
                    height,
                    tx_pos as i64,
                    raw_json,
                    charm.data.clone(),
                    confirmations as i32,
                    is_confirmed,
                    blockchain.to_string(),
                    network.to_string(),
                );

                let charm_item = (
                    txid.to_string(),
                    charm.charmid,
                    height,
                    charm.data.clone(),
                    charm.asset_type.clone(),
                    blockchain.to_string(),
                    network.to_string(),
                );

                // Assets are now created directly by the charm service during native parsing
                // No need to extract app_id here as it's handled in the native charm parser
                let asset_item = None;

                Ok(Some((transaction_item, charm_item, asset_item)))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                logging::log_error(&format!(
                    "[{}] Error processing potential charm {}: {}",
                    network_id.name, txid, e
                ));
                Ok(None)
            }
        }
    }

    /// Get optimized parallel configuration based on provider capabilities
    fn get_parallel_config_for_provider(&self, tx_count: usize) -> ParallelConfig {
        // Check if we have external providers that might require fallback
        let has_external_providers = self.has_external_providers();
        
        if has_external_providers {
            // QuickNode optimized config - can handle 10 req/s
            ParallelConfig {
                max_concurrent_requests: 8,   // Increased for better throughput
                requests_per_second: 8,       // Near QuickNode limit of 10 req/s
                batch_size: 20,               // Larger batches
                request_timeout_ms: 15000,    // Reasonable timeout
            }
        } else {
            // Only local providers: Use aggressive configuration
            ParallelConfig {
                max_concurrent_requests: if tx_count > 1000 { 100 } else if tx_count > 500 { 75 } else { 50 },
                requests_per_second: 100,
                batch_size: if tx_count > 2000 { 200 } else if tx_count > 500 { 100 } else { 50 },
                request_timeout_ms: 10000,
            }
        }
    }
    
    /// Check if we have any external providers (QuickNode) in the provider list
    fn has_external_providers(&self) -> bool {
        self.bitcoin_client.has_external_providers()
    }

    /// Heuristic to determine if we're likely using a local Bitcoin node
    fn is_using_local_node(&self) -> bool {
        self.bitcoin_client.is_using_local_node()
    }

}
