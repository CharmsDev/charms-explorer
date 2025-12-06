//! Block processor for handling individual block processing operations

use bitcoincore_rpc::bitcoin;
use futures::stream::{self, StreamExt};
use serde_json::json;

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::BitcoinClient;
use crate::infrastructure::persistence::repositories::{
    BookmarkRepository, SummaryRepository, TransactionRepository,
};
use crate::utils::logging;

use super::batch_processor::{
    AssetBatchItem, BatchProcessor, CharmBatchItem, TransactionBatchItem,
};
use super::retry_handler::RetryHandler;

/// Handles processing of individual blocks
#[derive(Debug)]
pub struct BlockProcessor {
    bitcoin_client: BitcoinClient,
    charm_service: CharmService,
    bookmark_repository: BookmarkRepository,
    transaction_repository: TransactionRepository,
    summary_repository: SummaryRepository,
    retry_handler: RetryHandler,
}

impl BlockProcessor {
    pub fn new(
        bitcoin_client: BitcoinClient,
        charm_service: CharmService,
        bookmark_repository: BookmarkRepository,
        transaction_repository: TransactionRepository,
        summary_repository: SummaryRepository,
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

        // 1. Fetch Block Hash
        let block_hash = self.get_block_hash(height, network_id).await?;

        // 2. Fetch Block Data
        let block = self.get_block(&block_hash, network_id).await?;

        // 3. Process Transactions (CPU Parallel)
        let (transaction_batch, charm_batch, asset_batch) = self
            .process_transactions(&block, &block_hash, height, latest_height, network_id)
            .await?;

        let batch_processor = BatchProcessor::new(
            self.charm_service.clone(),
            self.transaction_repository.clone(),
        );

        // 4. Save Transactions (DB)
        if !transaction_batch.is_empty() {
            batch_processor
                .save_transaction_batch(transaction_batch.clone(), height, network_id)
                .await?;
        }

        // 5. Save Charms (Queue)
        if !charm_batch.is_empty() {
            batch_processor
                .save_charm_batch(charm_batch.clone(), height, network_id)
                .await?;
        }

        // 6. Save Assets (DB)
        if !asset_batch.is_empty() {
            batch_processor
                .save_asset_batch(asset_batch.clone(), height, network_id)
                .await?;
        }

        // 7. Mark Spent Charms (DB)
        self.mark_spent_charms(&block, network_id).await?;

        // Update summary table with current statistics
        let summary_updater = super::SummaryUpdater::new(
            self.bitcoin_client.clone(),
            self.summary_repository.clone(),
        );
        summary_updater
            .update_statistics(
                height,
                latest_height,
                &charm_batch,
                &transaction_batch,
                network_id,
            )
            .await?;

        // Save bookmark ONLY after all processing is complete
        // This ensures we don't skip blocks if interrupted mid-processing
        self.save_bookmark(&block_hash, height, latest_height, network_id)
            .await?;

        // Single summary line per block with progress
        let remaining = latest_height.saturating_sub(height);
        logging::log_info(&format!(
            "[{}] ✅ Block {}: Tx {} | Charms {} ({} remaining)",
            network_id.name,
            height,
            block.txdata.len(),
            charm_batch.len(),
            remaining
        ));

        // [RJJ-DEBUG] PAUSE when BRO NFT is detected (n/3d7fe7e4...) - DISABLED FOR NOW
        // This allows manual verification of NFT metadata extraction before continuing
        let _has_bro_nft = charm_batch.iter().any(|(_, _, _, _, asset_type, _, _, app_id, _)| {
            asset_type == "nft" && app_id.starts_with("n/3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b")
        });
        
        if false {
            logging::log_warning(&format!(
                "\n\n🛑 ============================================\n\
                 🛑 PAUSA FORZADA - Bloque 913084 procesado\n\
                 🛑 ============================================\n\
                 \n\
                 Este es el bloque donde aparece el NFT de referencia del token BRO.\n\
                 \n\
                 📊 VERIFICAR EN LA BASE DE DATOS:\n\
                 \n\
                 1. Supply correcto en assets:\n\
                    SELECT app_id, asset_type, total_supply, data->>'supply' as data_supply\n\
                    FROM assets WHERE app_id LIKE 'n/3d7f%' OR app_id LIKE 't/3d7f%';\n\
                 \n\
                 2. Metadata del NFT:\n\
                    SELECT app_id, name, symbol, description, image_url\n\
                    FROM assets WHERE asset_type = 'nft';\n\
                 \n\
                 3. Charms guardados:\n\
                    SELECT COUNT(*) FROM charms WHERE block_height = 913084;\n\
                 \n\
                 ⏸️  El indexer continuará en 60 segundos...\n\
                 ⏸️  Presiona Ctrl+C para detener si necesitas más tiempo.\n\
                 \n\
                 ============================================\n"
            ));

            // Wait 60 seconds to allow manual verification
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

            logging::log_info(&format!(
                "[{}] ▶️  Continuando indexación desde bloque {}...",
                network_id.name,
                height + 1
            ));
        }

        Ok(())
    }

    /// Get block hash for given height
    async fn get_block_hash(
        &self,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<bitcoin::BlockHash, BlockProcessorError> {
        match self.bitcoin_client.get_block_hash(height).await {
            Ok(hash) => Ok(hash),
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
            Ok(block) => Ok(block),
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
    /// Optimized to avoid redundant RPC calls by using local block data
    async fn process_transactions(
        &self,
        block: &bitcoin::Block,
        _block_hash: &bitcoin::BlockHash,
        height: u64,
        latest_height: u64,
        network_id: &NetworkId,
    ) -> Result<
        (
            Vec<TransactionBatchItem>,
            Vec<CharmBatchItem>,
            Vec<AssetBatchItem>,
        ),
        BlockProcessorError,
    > {
        let blockchain = "Bitcoin".to_string();
        let network = network_id.name.clone();

        let charm_service = self.charm_service.clone(); // Clone owned service

        // Pre-process transactions to owned data to avoid lifetime issues with async stream
        let tx_data = Self::extract_transaction_data(block);

        // Create iterator of futures for parallel processing
        // We use owned data to ensure futures are 'static and Send
        let futures = tx_data.into_iter().map(|(txid, tx_hex, tx_pos, input_txids)| {
            let network_clone = network.clone();
            let blockchain_clone = blockchain.clone();

            let charm_service = charm_service.clone(); // Clone for this task

            async move {
                // [RJJ-DEBUG] Log specific transaction for BRO NFT
                if txid == "ea40035b7e6fed781f6c9c5c2929617594e66dde0e005be233165c31de9d6089" {
                    logging::log_warning(&format!(
                        "\n🔍 [DEBUG] Processing BRO NFT transaction:\n\
                         - TXID: {}\n\
                         - Block: {}\n\
                         - TX Hex length: {} bytes\n\
                         - Calling detect_and_process_charm_from_hex_with_latest...",
                        txid, height, tx_hex.len()
                    ));
                }
                
                // Use local hex to detect charm - NO RPC CALL
                let detection_result = charm_service
                    .detect_and_process_charm_from_hex_with_latest(
                        &txid,
                        height,
                        &tx_hex,
                        tx_pos,
                        latest_height,
                        input_txids,
                    )
                    .await;
                
                // [RJJ-DEBUG] Log result for BRO NFT transaction
                if txid == "ea40035b7e6fed781f6c9c5c2929617594e66dde0e005be233165c31de9d6089" {
                    match &detection_result {
                        Ok(Some(charm)) => {
                            logging::log_warning(&format!(
                                "✅ [DEBUG] BRO NFT DETECTED!\n\
                                 - App ID: {}\n\
                                 - Asset Type: {}\n\
                                 - Amount: {}\n\
                                 - Vout: {}\n\
                                 - Data: {:?}",
                                charm.app_id, charm.asset_type, charm.amount, charm.vout, charm.data
                            ));
                        },
                        Ok(None) => {
                            logging::log_warning(&format!(
                                "❌ [DEBUG] BRO NFT NOT DETECTED - Result was None\n\
                                 - This means the parser did not find a valid charm in the transaction"
                            ));
                        },
                        Err(e) => {
                            logging::log_warning(&format!(
                                "❌ [DEBUG] BRO NFT DETECTION ERROR: {:?}",
                                e
                            ));
                        }
                    }
                }
                
                match detection_result {
                    Ok(Some(charm)) => {
                        let confirmations = latest_height - height + 1;
                        let is_confirmed = confirmations >= 6;

                        let raw_json = json!({
                            "hex": tx_hex,
                            "txid": txid,
                        });

                        let transaction_item = (
                            txid.clone(),
                            height,
                            tx_pos as i64,
                            raw_json,
                            charm.data.clone(),
                            confirmations as i32,
                            is_confirmed,
                            blockchain_clone.clone(),
                            network_clone.clone(),
                        );

                        let charm_item = (
                            txid,
                            charm.vout,
                            height,
                            charm.data.clone(),
                            charm.asset_type.clone(),
                            blockchain_clone,
                            network_clone,
                            charm.app_id.clone(),
                            charm.amount,
                        );

                        Some((transaction_item, charm_item, None))
                    }
                    Ok(None) => None,
                    Err(e) => {
                        logging::log_error(&format!(
                            "[{}] Error processing potential charm {}: {}",
                            network_clone, txid, e
                        ));
                        None
                    }
                }
            }
        });

        // Determine batch size from environment or default to safe value for small servers
        let batch_size = std::env::var("INDEXER_BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(50); // Default 50 is safe for 1GB RAM / 1 vCPU

        // Process transactions in parallel
        // This uses the async runtime to interleave I/O and CPU tasks efficiently
        let results: Vec<Option<(TransactionBatchItem, CharmBatchItem, Option<AssetBatchItem>)>> =
            stream::iter(futures)
                .buffer_unordered(batch_size)
                .collect()
                .await;

        // Collect results into batches
        let mut transaction_batch = Vec::with_capacity(results.len());
        let mut charm_batch = Vec::with_capacity(results.len());
        let mut asset_batch = Vec::new();

        for result in results {
            if let Some((tx_item, charm_item, asset_item)) = result {
                transaction_batch.push(tx_item);
                charm_batch.push(charm_item);
                if let Some(asset) = asset_item {
                    asset_batch.push(asset);
                }
            }
        }

        Ok((transaction_batch, charm_batch, asset_batch))
    }

    /// Mark charms as spent by analyzing transaction inputs in the block
    async fn mark_spent_charms(
        &self,
        block: &bitcoin::Block,
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {
        // Collect all input txids (UTXOs being spent) from all transactions in the block
        let mut spent_txids = Vec::new();

        for tx in &block.txdata {
            // Skip coinbase transactions (they don't spend existing UTXOs)
            if tx.is_coin_base() {
                continue;
            }

            // Extract the txid from each input (previous output being spent)
            for input in &tx.input {
                let prev_txid = input.previous_output.txid.to_string();
                spent_txids.push(prev_txid);
            }
        }

        // Mark all collected txids as spent in batch using CharmService
        if !spent_txids.is_empty() {
            self.retry_handler
                .execute_with_retry_and_logging(
                    || async {
                        self.charm_service
                            .mark_charms_as_spent_batch(spent_txids.clone())
                            .await
                            .map_err(|e| {
                                crate::infrastructure::persistence::error::DbError::QueryError(
                                    e.to_string(),
                                )
                            })
                    },
                    "mark charms as spent",
                    &network_id.name,
                )
                .await
                .map_err(BlockProcessorError::DbError)?;
        }

        Ok(())
    }

    /// Extracts transaction data into an owned vector to avoid lifetime issues
    /// Returns: (txid, tx_hex, tx_pos, input_txids)
    fn extract_transaction_data(block: &bitcoin::Block) -> Vec<(String, String, usize, Vec<String>)> {
        block
            .txdata
            .iter()
            .enumerate()
            .map(|(tx_pos, tx)| {
                // Extract input txids (previous outputs being spent)
                let input_txids: Vec<String> = tx
                    .input
                    .iter()
                    .filter(|input| !input.previous_output.is_null()) // Skip coinbase
                    .map(|input| input.previous_output.txid.to_string())
                    .collect();
                
                (
                    tx.txid().to_string(),
                    bitcoin::consensus::encode::serialize_hex(tx),
                    tx_pos,
                    input_txids,
                )
            })
            .collect()
    }
}
