//! Block processor for handling individual block processing operations

use bitcoincore_rpc::bitcoin;
use serde_json::json;

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::BitcoinClient;
use crate::infrastructure::persistence::repositories::{BookmarkRepository, SummaryRepository, TransactionRepository};
use crate::utils::logging;

use super::batch_processor::{BatchProcessor, CharmBatchItem, TransactionBatchItem};
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
        // Log every block being processed
        logging::log_info(&format!(
            "[{}] 📦 Processing block: {}",
            network_id.name, height
        ));

        // Get latest height once for the entire block processing
        let latest_height = self.bitcoin_client.get_block_count().map_err(|e| {
            logging::log_error(&format!(
                "[{}] Error getting block count: {}",
                network_id.name, e
            ));
            BlockProcessorError::BitcoinClientError(e)
        })?;

        let block_hash = self.get_block_hash(height, network_id).await?;
        logging::log_info(&format!(
            "[{}] 🔍 Got block hash for {}: {}",
            network_id.name, height, block_hash
        ));

        let block = self.get_block(&block_hash, network_id).await?;
        logging::log_info(&format!(
            "[{}] 📄 Got block data for {}: {} transactions",
            network_id.name,
            height,
            block.txdata.len()
        ));

        self.save_bookmark(&block_hash, height, latest_height, network_id).await?;

        let (transaction_batch, charm_batch) = self
            .process_transactions(&block, &block_hash, height, latest_height, network_id)
            .await?;

        let batch_processor = BatchProcessor::new(self.charm_service, self.transaction_repository);

        batch_processor
            .save_transaction_batch(transaction_batch.clone(), height, network_id)
            .await?;

        batch_processor
            .save_charm_batch(charm_batch.clone(), height, network_id)
            .await?;

        // Update summary table with current statistics
        let summary_updater = super::SummaryUpdater::new(self.bitcoin_client, self.summary_repository);
        summary_updater.update_statistics(height, latest_height, &charm_batch, &transaction_batch, network_id)
            .await?;

        logging::log_info(&format!(
            "[{}] ✅ Completed processing block: {}",
            network_id.name, height
        ));

        Ok(())
    }

    /// Get block hash for given height
    async fn get_block_hash(
        &self,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<bitcoin::BlockHash, BlockProcessorError> {
        self.bitcoin_client.get_block_hash(height).map_err(|e| {
            logging::log_error(&format!(
                "[{}] Error getting block hash for height {}: {}",
                network_id.name, height, e
            ));
            BlockProcessorError::BitcoinClientError(e)
        })
    }

    /// Get block data for given hash
    async fn get_block(
        &self,
        block_hash: &bitcoin::BlockHash,
        network_id: &NetworkId,
    ) -> Result<bitcoin::Block, BlockProcessorError> {
        self.bitcoin_client.get_block(block_hash).map_err(|e| {
            logging::log_error(&format!(
                "[{}] Error getting block for hash {}: {}",
                network_id.name, block_hash, e
            ));
            BlockProcessorError::BitcoinClientError(e)
        })
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

        logging::log_info(&format!(
            "[{}] 💾 Saving bookmark for block {} (hash: {}, confirmed: {})",
            network_id.name, height, block_hash, is_confirmed
        ));

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

    /// Process all transactions in a block
    async fn process_transactions(
        &self,
        block: &bitcoin::Block,
        block_hash: &bitcoin::BlockHash,
        height: u64,
        latest_height: u64,
        network_id: &NetworkId,
    ) -> Result<(Vec<TransactionBatchItem>, Vec<CharmBatchItem>), BlockProcessorError> {
        let mut transaction_batch = Vec::new();
        let mut charm_batch = Vec::new();

        let blockchain = "Bitcoin".to_string();
        let network = network_id.name.clone();

        logging::log_info(&format!(
            "[{}] 🔄 Processing {} transactions in block {}",
            network_id.name,
            block.txdata.len(),
            height
        ));

        for (tx_pos, tx) in block.txdata.iter().enumerate() {
            let txid = tx.txid();
            let txid_str = txid.to_string();

            // Log every 100th transaction or all transactions if block has <= 50 txs
            if tx_pos % 100 == 0 || block.txdata.len() <= 50 {
                logging::log_info(&format!(
                    "[{}] 📄 Processing tx {}/{} in block {}",
                    network_id.name,
                    tx_pos + 1,
                    block.txdata.len(),
                    height
                ));
            }

            if let Some((transaction_item, charm_item)) = self
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
            }
        }

        logging::log_info(&format!(
            "[{}] 🎯 Found {} charms in block {} ({} total transactions)",
            network_id.name,
            charm_batch.len(),
            height,
            block.txdata.len()
        ));

        Ok((transaction_batch, charm_batch))
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
    ) -> Result<Option<(TransactionBatchItem, CharmBatchItem)>, BlockProcessorError> {
        let raw_tx_hex = match self
            .bitcoin_client
            .get_raw_transaction_hex(txid, Some(block_hash))
        {
            Ok(hex) => hex,
            Err(e) => {
                logging::log_error(&format!(
                    "[{}] Error getting raw transaction {}: {}",
                    network_id.name, txid, e
                ));
                return Ok(None);
            }
        };

        match self
            .charm_service
            .detect_and_process_charm(txid, height, Some(block_hash))
            .await
        {
            Ok(Some(charm)) => {
                logging::log_info(&format!(
                    "[{}] 🎉 Block {}: Found charm tx: {} at pos {} (charm_id: {})",
                    network_id.name, height, txid, tx_pos, charm.charmid
                ));

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
                    charm.data,
                    charm.asset_type,
                    blockchain.to_string(),
                    network.to_string(),
                );

                Ok(Some((transaction_item, charm_item)))
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

}
