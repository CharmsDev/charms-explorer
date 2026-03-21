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
    /// Updates stats_holders ONLY for charms that were actually inserted (not
    /// mempool-promoted duplicates that already had stats_holders updated).
    pub async fn save_charm_batch(
        &self,
        batch: Vec<CharmBatchItem>,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {
        if batch.is_empty() {
            return Ok(());
        }

        // Save charms — returns which (txid, vout) were actually new insertions
        let inserted = self
            .charm_service
            .save_batch(batch.clone())
            .await
            .map_err(|e| {
                BlockProcessorError::ProcessingError(format!("Failed to save charm batch: {}", e))
            })?;

        // Build a set of inserted keys for fast lookup
        let inserted_set: std::collections::HashSet<(String, i32)> =
            inserted.into_iter().collect();

        // Only update stats_holders for charms that were actually NEW insertions.
        // Charms promoted from mempool (duplicates skipped by ON CONFLICT DO NOTHING)
        // already had their stats_holders updated during mempool detection.
        let holder_updates: Vec<(String, String, i64, i32)> = batch
            .iter()
            .filter_map(
                |(txid, vout, block_height, _, _, _, _, address, app_id, amount, _)| {
                    // Skip charms that were not inserted (already existed from mempool)
                    if !inserted_set.contains(&(txid.clone(), *vout)) {
                        return None;
                    }
                    if let Some(addr) = address {
                        if *amount > 0 && !addr.is_empty() {
                            if app_id.starts_with("t/") {
                                let nft_app_id = app_id.replacen("t/", "n/", 1);
                                return Some((
                                    nft_app_id,
                                    addr.clone(),
                                    *amount,
                                    *block_height as i32,
                                ));
                            } else if app_id.starts_with("n/") {
                                return Some((
                                    app_id.clone(),
                                    addr.clone(),
                                    1_i64,
                                    *block_height as i32,
                                ));
                            }
                        }
                    }
                    None
                },
            )
            .collect();

        if !holder_updates.is_empty() {
            if let Err(e) = self
                .charm_service
                .get_stats_holders_repository()
                .update_holders_batch(holder_updates)
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
    String, // txid
    u64,    // height
    i64,    // position
    Value,  // raw_json
    Value,  // charm_data
    i32,    // confirmations
    bool,   // is_confirmed
    String, // blockchain
    String, // network
);

/// Charm batch item for bulk operations
pub type CharmBatchItem = (
    String,         // txid
    i32,            // vout
    u64,            // height
    Value,          // data
    String,         // asset_type
    String,         // blockchain
    String,         // network
    Option<String>, // address
    String,         // app_id
    i64,            // amount
    Option<String>, // tags
);

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
);
