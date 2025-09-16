//! Batch processor for handling bulk operations on charms and transactions

use serde_json::Value;

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::persistence::repositories::TransactionRepository;
use crate::utils::logging;

/// Handles batch processing of charms and transactions
#[derive(Debug)]
pub struct BatchProcessor<'a> {
    charm_service: &'a CharmService,
    transaction_repository: &'a TransactionRepository,
}

impl<'a> BatchProcessor<'a> {
    pub fn new(
        charm_service: &'a CharmService,
        transaction_repository: &'a TransactionRepository,
    ) -> Self {
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

        logging::log_info(&format!(
            "[{}] Saving batch of {} transactions for block {}",
            network_id.name,
            batch.len(),
            height
        ));

        let mut retry_count = 0;
        let max_retries = 3;

        loop {
            match self.transaction_repository.save_batch(batch.clone()).await {
                Ok(_) => {
                    logging::log_info(&format!(
                        "[{}] Successfully saved transaction batch for block {}",
                        network_id.name, height
                    ));
                    return Ok(());
                }
                Err(e) => {
                    retry_count += 1;
                    logging::log_error(&format!(
                        "[{}] Error saving transaction batch for block {} (attempt {}/{}): {}",
                        network_id.name, height, retry_count, max_retries, e
                    ));

                    if retry_count >= max_retries {
                        return Err(BlockProcessorError::DbError(e));
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }
    }

    /// Save charm batch with retry logic
    pub async fn save_charm_batch(
        &self,
        batch: Vec<CharmBatchItem>,
        height: u64,
        network_id: &NetworkId,
    ) -> Result<(), BlockProcessorError> {
        if batch.is_empty() {
            return Ok(());
        }

        logging::log_info(&format!(
            "[{}] Saving batch of {} charms for block {}",
            network_id.name,
            batch.len(),
            height
        ));

        let mut retry_count = 0;
        let max_retries = 3;

        loop {
            match self.charm_service.save_batch(batch.clone()).await {
                Ok(_) => {
                    logging::log_info(&format!(
                        "[{}] Successfully saved charm batch for block {}",
                        network_id.name, height
                    ));
                    return Ok(());
                }
                Err(e) => {
                    retry_count += 1;
                    logging::log_error(&format!(
                        "[{}] Error saving charm batch for block {} (attempt {}/{}): {}",
                        network_id.name, height, retry_count, max_retries, e
                    ));

                    if retry_count >= max_retries {
                        return Err(BlockProcessorError::ProcessingError(format!(
                            "Failed to save charm batch: {}",
                            e
                        )));
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }
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
    String, // txid
    String, // charmid
    u64,    // height
    Value,  // data
    String, // asset_type
    String, // blockchain
    String, // network
);
