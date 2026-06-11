//! Batch processor for handling bulk operations on charms and transactions

use serde_json::Value;

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::persistence::repositories::TransactionRepository;
use crate::utils::logging;

/// Handles batch processing of charms and transactions
#[derive(Debug)]
pub struct BatchProcessor {
    charm_service: CharmService,
    transaction_repository: TransactionRepository,
}

impl BatchProcessor {
    pub fn new(charm_service: CharmService, transaction_repository: TransactionRepository) -> Self {
        Self {
            charm_service,
            transaction_repository,
        }
    }

    /// Save transaction batch with retry logic
    pub async fn save_transaction_batch(
        &self,
        batch: Vec<TransactionBatchItem>,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {
        if batch.is_empty() {
            return Ok(());
        }

        self.execute_batch_save(
            "transaction",
            batch.len(),
            height,
            network_id,
            || async { self.transaction_repository.save_batch(batch.clone()).await },
            |e| BlockProcessorError::DbError(e),
        )
        .await
    }

    /// Save charm batch with retry logic.
    /// Updates stats_holders for ALL charms in the block.
    /// This is safe because mempool never touches stats_holders — it only tracks
    /// confirmed balances. Whether a charm was new or promoted from mempool,
    /// the block processor is the single writer to stats_holders.
    pub async fn save_charm_batch(
        &self,
        batch: Vec<CharmBatchItem>,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {
        if batch.is_empty() {
            return Ok(());
        }

        // Compute stats_holders updates BEFORE consuming the batch.
        // Safe: mempool never updates stats_holders, so there's no double-counting.
        // For tokens (t/): use actual amount as balance delta.
        // For NFTs   (n/): use 1 as balance delta (ownership count).
        let holder_updates: Vec<(String, String, i64, i32)> = batch
            .iter()
            .filter_map(|c| {
                let addr = c.address.as_ref()?;
                if c.amount <= 0 || addr.is_empty() {
                    return None;
                }
                if c.app_id.starts_with("t/") {
                    Some((
                        crate::domain::services::app_id::token_to_nft(&c.app_id),
                        addr.clone(),
                        c.amount,
                        c.block_height as i32,
                    ))
                } else if c.app_id.starts_with("n/") {
                    Some((c.app_id.clone(), addr.clone(), 1_i64, c.block_height as i32))
                } else {
                    None
                }
            })
            .collect();

        // Save charms (ON CONFLICT DO NOTHING handles duplicates).
        // Repos still consume the historical tuple shape; convert at the boundary.
        let tuples: Vec<_> = batch.into_iter().map(CharmBatchItem::into_tuple).collect();
        let _inserted = self
            .charm_service
            .save_batch(tuples)
            .await
            .map_err(|e| {
                BlockProcessorError::ProcessingError(format!("Failed to save charm batch: {}", e))
            })?;

        if !holder_updates.is_empty() {
            if let Err(e) = self
                .charm_service
                .get_stats_holders_repository()
                .update_holders_batch(holder_updates, &network_id.name)
                .await
            {
                logging::log_warning(&format!(
                    "[{}] Failed to update stats_holders for new charms at block {}: {}",
                    network_id.name, height, e
                ));
            }
        }

        Ok(())
    }

    /// Save asset batch with retry logic
    pub async fn save_asset_batch(
        &self,
        batch: Vec<AssetBatchItem>,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {
        if batch.is_empty() {
            return Ok(());
        }

        self.execute_batch_save(
            "asset",
            batch.len(),
            height,
            network_id,
            || async { self.charm_service.save_asset_batch(batch.clone()).await },
            |e| BlockProcessorError::ProcessingError(format!("Failed to save asset batch: {}", e)),
        )
        .await
    }

    /// Generic batch save execution with retry logic
    async fn execute_batch_save<F, Fut, E, ErrMapper>(
        &self,
        batch_type: &str,
        _batch_size: usize,
        height: u64,
        network_id: &NetworkId,
        operation: F,
        error_mapper: ErrMapper,
    ) -> Result<(), BlockProcessorError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<(), E>>,
        ErrMapper: Fn(E) -> BlockProcessorError,
        E: std::fmt::Debug,
    {
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY_MS: u64 = 500;

        for attempt in 1..=MAX_RETRIES {
            match operation().await {
                Ok(_) => {
                    return Ok(());
                }
                Err(e) => {
                    logging::log_error(&format!(
                        "[{}] Error saving {} batch for block {} (attempt {}/{}): {:?}",
                        network_id.name, batch_type, height, attempt, MAX_RETRIES, e
                    ));

                    if attempt >= MAX_RETRIES {
                        return Err(error_mapper(e));
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;
                }
            }
        }

        unreachable!("Loop should have returned or errored")
    }
}

/// Transaction batch item for bulk operations
pub type TransactionBatchItem = (
    String,         // txid
    u64,            // height
    i64,            // position
    Value,          // raw_json
    Value,          // charm_data
    i32,            // confirmations
    bool,           // is_confirmed
    String,         // blockchain
    String,         // network
    Option<String>, // tags
    Option<String>, // tx_type
);

/// Charm batch item for bulk operations.
#[derive(Debug, Clone)]
pub struct CharmBatchItem {
    pub txid: String,
    pub vout: i32,
    pub block_height: u64,
    pub data: Value,
    pub asset_type: String,
    pub blockchain: String,
    pub network: String,
    pub address: Option<String>,
    pub app_id: String,
    pub amount: i64,
    pub tags: Option<String>,
}

impl CharmBatchItem {
    /// Repos still consume the historical 11-tuple shape; this preserves
    /// the wire format until they migrate too.
    pub fn into_tuple(
        self,
    ) -> (
        String,
        i32,
        u64,
        Value,
        String,
        String,
        String,
        Option<String>,
        String,
        i64,
        Option<String>,
    ) {
        (
            self.txid,
            self.vout,
            self.block_height,
            self.data,
            self.asset_type,
            self.blockchain,
            self.network,
            self.address,
            self.app_id,
            self.amount,
            self.tags,
        )
    }
}

/// Asset batch item for bulk operations
pub type AssetBatchItem = (
    String,         // app_id
    String,         // txid
    i32,            // vout
    u64,            // block_height
    String,         // asset_type
    u64,            // supply
    String,         // blockchain
    String,         // network
    Option<String>, // name
    Option<String>, // symbol
    Option<String>, // description
    Option<String>, // image_url
    Option<u8>,     // decimals
    Option<String>, // cardano_policy_id
    Option<String>, // cardano_asset_name
    Option<String>, // cardano_fingerprint
);
