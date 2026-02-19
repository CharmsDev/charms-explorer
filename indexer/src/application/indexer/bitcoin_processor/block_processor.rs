//! Block processor for handling individual block processing operations
//! Implements a linear, synchronous flow per block:
//! 1. Detect charms from transactions
//! 2. Save transactions
//! 3. Save charms
//! 4. Save assets
//! 5. Mark spent charms
//! 6. Update statistics

use bitcoincore_rpc::bitcoin;
use serde_json::json;

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::BitcoinClient;
use crate::infrastructure::persistence::repositories::utxo_repository::UtxoInsert;
use crate::infrastructure::persistence::repositories::{
    BlockStatusRepository, SummaryRepository, TransactionRepository, UtxoRepository,
};
use crate::utils::logging;

use super::batch_processor::{
    AssetBatchItem, BatchProcessor, CharmBatchItem, TransactionBatchItem,
};
use super::retry_handler::RetryHandler;

/// Handles processing of individual blocks
/// Unified processor: works with both live node data and cached transactions
#[derive(Debug)]
pub struct BlockProcessor {
    bitcoin_client: BitcoinClient,
    charm_service: CharmService,
    transaction_repository: TransactionRepository,
    summary_repository: SummaryRepository,
    block_status_repository: BlockStatusRepository,
    utxo_repository: UtxoRepository,
    retry_handler: RetryHandler,
}

impl BlockProcessor {
    pub fn new(
        bitcoin_client: BitcoinClient,
        charm_service: CharmService,
        transaction_repository: TransactionRepository,
        summary_repository: SummaryRepository,
        block_status_repository: BlockStatusRepository,
        utxo_repository: UtxoRepository,
    ) -> Self {
        Self {
            bitcoin_client,
            charm_service,
            transaction_repository,
            summary_repository,
            block_status_repository,
            utxo_repository,
            retry_handler: RetryHandler::new(),
        }
    }

    /// Process a single block with linear synchronous flow:
    /// detect → save transactions → save charms → save assets → mark spent → update stats
    pub async fn process_block(
        &self,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {
        let latest_height = self
            .bitcoin_client
            .get_block_count()
            .await
            .map_err(|e| BlockProcessorError::BitcoinClientError(e))?;

        // Fetch block from node
        let block_hash = self.get_block_hash(height, network_id).await?;
        let block = self.get_block(&block_hash, network_id).await?;

        // STEP 1: Detect charms from all transactions (no DB writes)
        let (transaction_batch, charm_batch, asset_batch) = self
            .detect_charms_from_block(&block, height, latest_height, network_id)
            .await?;

        let batch_processor = BatchProcessor::new(
            self.charm_service.clone(),
            self.transaction_repository.clone(),
        );

        // STEP 2: Save transactions
        if !transaction_batch.is_empty() {
            batch_processor
                .save_transaction_batch(transaction_batch.clone(), height, network_id)
                .await?;
        }

        // STEP 3: Save charms
        if !charm_batch.is_empty() {
            batch_processor
                .save_charm_batch(charm_batch.clone(), height, network_id)
                .await?;
        }

        // STEP 4: Save assets
        if !asset_batch.is_empty() {
            batch_processor
                .save_asset_batch(asset_batch.clone(), height, network_id)
                .await?;
        }

        // STEP 5: Mark spent charms
        self.mark_spent_charms(&block, network_id).await?;

        // STEP 5.5: Update address UTXO index
        self.update_utxo_index(&block, height, network_id).await?;

        // STEP 6: Update summary statistics
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

        // STEP 7: Update block_status
        let confirmations = latest_height.saturating_sub(height) + 1;
        let is_confirmed = confirmations >= 6;

        let _ = self
            .block_status_repository
            .mark_downloaded(
                height as i32,
                Some(&block_hash.to_string()),
                block.txdata.len() as i32,
                network_id,
            )
            .await;

        let _ = self
            .block_status_repository
            .mark_processed(height as i32, charm_batch.len() as i32, network_id)
            .await;

        if is_confirmed {
            let _ = self
                .block_status_repository
                .mark_confirmed(height as i32, network_id)
                .await;
        }

        // Log progress
        let remaining = latest_height.saturating_sub(height);
        logging::log_info(&format!(
            "[{}] ✅ Block {}: Tx {} | Charms {} ({} remaining)",
            network_id.name,
            height,
            block.txdata.len(),
            charm_batch.len(),
            remaining
        ));

        Ok(())
    }

    /// Process a block from cached transactions in database (reindex mode)
    /// Uses data from transactions table instead of fetching from Bitcoin node
    pub async fn process_block_from_cache(
        &self,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {
        // Get cached transactions for this block
        let cached_txs = self
            .transaction_repository
            .find_by_block_height(height)
            .await
            .map_err(|e| BlockProcessorError::ProcessingError(format!("DB error: {}", e)))?;

        if cached_txs.is_empty() {
            // No transactions in cache, mark as processed
            let _ = self
                .block_status_repository
                .mark_processed(height as i32, 0, network_id)
                .await;
            return Ok(());
        }

        // Get latest height for confirmations calculation
        let _latest_height = self
            .bitcoin_client
            .get_block_count()
            .await
            .unwrap_or(height);

        let mut charm_count = 0;

        // Process each cached transaction - reprocess charms
        for tx in &cached_txs {
            // Extract hex from raw JSON
            let tx_hex = tx.raw.get("hex").and_then(|v| v.as_str()).unwrap_or("");

            if tx_hex.is_empty() {
                continue;
            }

            // Use the charm service to detect charm
            match self
                .charm_service
                .detect_and_process_charm_from_hex(&tx.txid, height, tx_hex, tx.ordinal as usize)
                .await
            {
                Ok(Some(_)) => {
                    charm_count += 1;
                }
                Ok(None) => {}
                Err(e) => {
                    logging::log_error(&format!(
                        "[{}] Error processing tx {} at height {}: {}",
                        network_id.name, tx.txid, height, e
                    ));
                }
            }
        }

        // Mark spent charms by fetching the full block from the node
        // This is necessary because non-charm transactions can also spend charm UTXOs
        match self.get_block_hash(height, network_id).await {
            Ok(block_hash) => match self.get_block(&block_hash, network_id).await {
                Ok(block) => {
                    if let Err(e) = self.mark_spent_charms(&block, network_id).await {
                        logging::log_warning(&format!(
                            "[{}] ⚠️ Failed to mark spent charms for reindex block {}: {}",
                            network_id.name, height, e
                        ));
                    }
                }
                Err(e) => {
                    logging::log_warning(&format!(
                        "[{}] ⚠️ Could not fetch block {} for spent tracking (pruned?): {}",
                        network_id.name, height, e
                    ));
                }
            },
            Err(e) => {
                logging::log_warning(&format!(
                    "[{}] ⚠️ Could not get block hash for height {} during reindex: {}",
                    network_id.name, height, e
                ));
            }
        }

        // Update block_status
        let _ = self
            .block_status_repository
            .mark_processed(height as i32, charm_count, network_id)
            .await;

        // Log progress
        logging::log_info(&format!(
            "[{}] ♻️ Reindex Block {}: Tx {} | Charms {}",
            network_id.name,
            height,
            cached_txs.len(),
            charm_count,
        ));

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

    /// Detect charms from all transactions in a block sequentially.
    /// Returns batch items for transactions, charms, and assets.
    /// No DB writes happen here — pure detection only.
    async fn detect_charms_from_block(
        &self,
        block: &bitcoin::Block,
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

        let tx_data = Self::extract_transaction_data(block);

        let mut transaction_batch = Vec::new();
        let mut charm_batch = Vec::new();
        let mut asset_batch: Vec<AssetBatchItem> = Vec::new();

        for (txid, tx_hex, tx_pos, input_txids) in tx_data {
            // Detect charm (no DB writes)
            let detection_result = self
                .charm_service
                .detect_charm_from_hex_with_context(
                    &txid,
                    height,
                    &tx_hex,
                    tx_pos,
                    latest_height,
                    input_txids,
                )
                .await;

            match detection_result {
                Ok(Some((charm, asset_requests))) => {
                    let confirmations = latest_height - height + 1;
                    let is_confirmed = confirmations >= 6;

                    let raw_json = json!({
                        "hex": tx_hex,
                        "txid": txid,
                    });

                    transaction_batch.push((
                        txid.clone(),
                        height,
                        tx_pos as i64,
                        raw_json,
                        charm.data.clone(),
                        confirmations as i32,
                        is_confirmed,
                        blockchain.clone(),
                        network.clone(),
                    ));

                    charm_batch.push((
                        txid.clone(),
                        charm.vout,
                        height,
                        charm.data.clone(),
                        charm.asset_type.clone(),
                        blockchain.clone(),
                        network.clone(),
                        charm.address.clone(),
                        charm.app_id.clone(),
                        charm.amount,
                        charm.tags.clone(),
                    ));

                    // Convert AssetSaveRequest to AssetBatchItem
                    for asset_req in asset_requests {
                        asset_batch.push((
                            asset_req.app_id,
                            txid.clone(),
                            charm.vout,
                            height,
                            asset_req.asset_type,
                            asset_req.supply,
                            asset_req.blockchain,
                            asset_req.network,
                            asset_req.name,
                            asset_req.symbol,
                            asset_req.description,
                            asset_req.image_url,
                            asset_req.decimals,
                        ));
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    logging::log_error(&format!(
                        "[{}] Error detecting charm in tx {}: {}",
                        network, txid, e
                    ));
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
        // Collect all input (txid, vout) pairs being spent from all transactions in the block
        let mut spent_txid_vouts: Vec<(String, i32)> = Vec::new();

        for tx in &block.txdata {
            // Skip coinbase transactions (they don't spend existing UTXOs)
            if tx.is_coin_base() {
                continue;
            }

            // Extract (txid, vout) from each input (previous output being spent)
            for input in &tx.input {
                let prev_txid = input.previous_output.txid.to_string();
                let prev_vout = input.previous_output.vout as i32;
                spent_txid_vouts.push((prev_txid, prev_vout));
            }
        }

        // Mark all collected (txid, vout) pairs as spent in batch using CharmService
        if !spent_txid_vouts.is_empty() {
            self.retry_handler
                .execute_with_retry_and_logging(
                    || async {
                        self.charm_service
                            .mark_charms_as_spent_batch(spent_txid_vouts.clone())
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

    /// Update the address UTXO index for a block:
    /// 1. Delete spent UTXOs (inputs)
    /// 2. Insert new UTXOs (outputs with valid addresses)
    async fn update_utxo_index(
        &self,
        block: &bitcoin::Block,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {
        let network_str = &network_id.name;
        let btc_network = match network_str.as_str() {
            "mainnet" => bitcoin::Network::Bitcoin,
            "testnet4" => bitcoin::Network::Testnet,
            _ => bitcoin::Network::Testnet,
        };

        // 1. Collect spent UTXOs from all inputs
        let mut spent: Vec<(String, i32)> = Vec::new();
        for tx in &block.txdata {
            if tx.is_coin_base() {
                continue;
            }
            for input in &tx.input {
                if !input.previous_output.is_null() {
                    spent.push((
                        input.previous_output.txid.to_string(),
                        input.previous_output.vout as i32,
                    ));
                }
            }
        }

        // 2. Collect new UTXOs from all outputs
        let mut new_utxos: Vec<UtxoInsert> = Vec::new();
        for tx in &block.txdata {
            let txid = tx.txid().to_string();
            for (vout, output) in tx.output.iter().enumerate() {
                // Skip unspendable outputs (OP_RETURN, etc.)
                if output.script_pubkey.is_provably_unspendable() {
                    continue;
                }
                // Try to extract address from script_pubkey
                if let Ok(address) =
                    bitcoin::Address::from_script(&output.script_pubkey, btc_network)
                {
                    new_utxos.push(UtxoInsert {
                        txid: txid.clone(),
                        vout: vout as i32,
                        address: address.to_string(),
                        value: output.value as i64,
                        script_pubkey: format!("{:x}", output.script_pubkey),
                        block_height: height as i32,
                        network: network_str.clone(),
                    });
                }
            }
        }

        // 3. Delete spent UTXOs
        if !spent.is_empty() {
            if let Err(e) = self
                .utxo_repository
                .delete_spent_batch(&spent, network_str)
                .await
            {
                logging::log_warning(&format!(
                    "[{}] Failed to delete spent UTXOs at block {}: {}",
                    network_str, height, e
                ));
            }
        }

        // 4. Insert new UTXOs
        if !new_utxos.is_empty() {
            if let Err(e) = self.utxo_repository.insert_batch(&new_utxos).await {
                logging::log_warning(&format!(
                    "[{}] Failed to insert UTXOs at block {}: {}",
                    network_str, height, e
                ));
            }
        }

        Ok(())
    }

    /// Extracts transaction data into an owned vector to avoid lifetime issues
    /// Returns: (txid, tx_hex, tx_pos, input_txids)
    fn extract_transaction_data(
        block: &bitcoin::Block,
    ) -> Vec<(String, String, usize, Vec<String>)> {
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
