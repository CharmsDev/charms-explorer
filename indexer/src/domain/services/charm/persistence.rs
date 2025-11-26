///! Charm persistence operations (batch saves)

use serde_json::Value;

use crate::domain::errors::CharmError;
use crate::infrastructure::persistence::repositories::{AssetRepository, CharmRepository};

/// Handles batch persistence operations for charms and assets
pub struct CharmPersistence<'a> {
    charm_repository: &'a CharmRepository,
    asset_repository: &'a AssetRepository,
}

impl<'a> CharmPersistence<'a> {
    pub fn new(
        charm_repository: &'a CharmRepository,
        asset_repository: &'a AssetRepository,
    ) -> Self {
        Self {
            charm_repository,
            asset_repository,
        }
    }

    /// Optimize DB session for high-throughput writer tasks
    pub async fn optimize_writer_session(&self) -> Result<(), CharmError> {
        self.charm_repository
            .set_synchronous_commit(false)
            .await
            .map_err(|e| CharmError::ProcessingError(format!(
                "Failed to set synchronous_commit off: {}",
                e
            )))
    }

    /// Saves multiple charms in a single database operation
    /// [RJJ-S01] Updated: replaced charmid with vout, added app_id and amount
    pub async fn save_charm_batch(
        &self,
        charms: Vec<(
            String,            // txid
            i32,               // vout
            u64,               // block_height
            serde_json::Value, // data
            String,            // asset_type
            String,            // blockchain
            String,            // network
            String,            // app_id
            i64,               // amount
        )>,
    ) -> Result<(), CharmError> {
        self.charm_repository
            .save_batch(charms)
            .await
            .map_err(|e| CharmError::ProcessingError(format!("Failed to save charm batch: {}", e)))
    }

    /// Save a batch of transactions to the repository
    /// 
    /// Note: This currently returns Ok() as transactions are handled separately
    /// TODO: Add TransactionRepository to CharmService dependencies if needed
    pub async fn save_transaction_batch(
        &self,
        _batch: Vec<(
            String,            // txid
            u64,               // block_height
            i64,               // tx_position
            serde_json::Value, // raw_json
            serde_json::Value, // charm_data
            i32,               // confirmations
            bool,              // is_confirmed
            String,            // blockchain
            String,            // network
        )>,
    ) -> Result<(), CharmError> {
        // Transactions are handled separately by TransactionRepository
        Ok(())
    }

    /// Save a batch of assets to the repository
    /// 
    /// Converts simplified asset batch items into the full tuple format expected by the repository
    pub async fn save_asset_batch(
        &self,
        batch: Vec<(
            String, // app_id
            String, // asset_type
            u64,    // supply
            String, // blockchain
            String, // network
        )>,
    ) -> Result<(), CharmError> {
        if batch.is_empty() {
            return Ok(());
        }

        // Transform simplified batch items into full repository format
        let asset_tuples: Vec<(String, String, i32, String, u64, Value, String, String, String)> = batch
            .into_iter()
            .map(|(app_id, asset_type, supply, blockchain, network)| {
                (
                    app_id.clone(),                          // app_id
                    String::new(),                           // txid - empty for asset records
                    0,                                       // vout_index - not applicable for assets
                    format!("charm-{}", app_id),             // charm_id derived from app_id
                    0,                                       // block_height - will be updated during processing
                    serde_json::json!({"supply": supply}),   // data with supply information
                    asset_type,                              // asset_type
                    blockchain,                              // blockchain
                    network,                                 // network
                )
            })
            .collect();

        self.asset_repository
            .save_batch(asset_tuples)
            .await
            .map_err(|e| CharmError::ProcessingError(format!("Failed to save asset batch: {}", e)))
    }
}
