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

        let tuples: Vec<_> = batch
            .into_iter()
            .map(TransactionBatchItem::into_tuple)
            .collect();
        self.execute_batch_save(
            "transaction",
            tuples.len(),
            height,
            network_id,
            || async { self.transaction_repository.save_batch(tuples.clone()).await },
            BlockProcessorError::DbError,
        )
        .await
    }

    /// Save charm batch with retry logic.
    ///
    /// Returns the positive holder deltas that the block processor must merge
    /// with the negative deltas coming from `mark_spent_charms` before
    /// calling `update_holders_batch` ONCE per block. Doing one merged update
    /// per (app_id, address) preserves the `last_updated_block < block` gate
    /// (crash-recovery safety) while still letting within-block adds + spends
    /// net correctly — the bug captured as anomaly A1 in the test report.
    pub async fn save_charm_batch(
        &self,
        batch: Vec<CharmBatchItem>,
        _height: u64,
        _network_id: &NetworkId,
    ) -> Result<Vec<(String, String, i64, i32)>, BlockProcessorError> {
        if batch.is_empty() {
            return Ok(Vec::new());
        }

        // Compute positive holder deltas. For tokens (t/) use the on-chain
        // amount; for NFTs (n/) use 1 (ownership count). All deltas carry
        // the block height so the gate at the repo layer can advance.
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

        Ok(holder_updates)
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

        let tuples: Vec<_> = batch.into_iter().map(AssetBatchItem::into_tuple).collect();
        self.execute_batch_save(
            "asset",
            tuples.len(),
            height,
            network_id,
            || async { self.charm_service.save_asset_batch(tuples.clone()).await },
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
/// Transaction batch item for bulk operations.
#[derive(Debug, Clone)]
pub struct TransactionBatchItem {
    pub txid: String,
    pub block_height: u64,
    pub position: i64,
    pub raw_json: Value,
    pub charm_data: Value,
    pub confirmations: i32,
    pub is_confirmed: bool,
    pub blockchain: String,
    pub network: String,
    pub tags: Option<String>,
    pub tx_type: Option<String>,
}

impl TransactionBatchItem {
    /// Repos still consume the historical tuple shape; preserves wire format.
    #[allow(clippy::type_complexity)]
    pub fn into_tuple(
        self,
    ) -> (
        String,
        u64,
        i64,
        Value,
        Value,
        i32,
        bool,
        String,
        String,
        Option<String>,
        Option<String>,
    ) {
        (
            self.txid,
            self.block_height,
            self.position,
            self.raw_json,
            self.charm_data,
            self.confirmations,
            self.is_confirmed,
            self.blockchain,
            self.network,
            self.tags,
            self.tx_type,
        )
    }
}

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
    #[allow(clippy::type_complexity)]
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

/// Asset batch item for bulk operations.
#[derive(Debug, Clone)]
pub struct AssetBatchItem {
    pub app_id: String,
    pub txid: String,
    pub vout: i32,
    pub block_height: u64,
    pub asset_type: String,
    pub supply: u64,
    pub blockchain: String,
    pub network: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub decimals: Option<u8>,
    pub cardano_policy_id: Option<String>,
    pub cardano_asset_name: Option<String>,
    pub cardano_fingerprint: Option<String>,
}

impl AssetBatchItem {
    /// Repos still consume the historical tuple shape; preserves wire format.
    #[allow(clippy::type_complexity)]
    pub fn into_tuple(
        self,
    ) -> (
        String,
        String,
        i32,
        u64,
        String,
        u64,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<u8>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        (
            self.app_id,
            self.txid,
            self.vout,
            self.block_height,
            self.asset_type,
            self.supply,
            self.blockchain,
            self.network,
            self.name,
            self.symbol,
            self.description,
            self.image_url,
            self.decimals,
            self.cardano_policy_id,
            self.cardano_asset_name,
            self.cardano_fingerprint,
        )
    }
}
