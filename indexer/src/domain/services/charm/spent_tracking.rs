///! Spent UTXO tracking for charms

use crate::domain::errors::CharmError;
use crate::infrastructure::persistence::repositories::CharmRepository;

/// Handles marking charms as spent when their UTXOs are consumed
pub struct SpentTracker<'a> {
    charm_repository: &'a CharmRepository,
}

impl<'a> SpentTracker<'a> {
    pub fn new(charm_repository: &'a CharmRepository) -> Self {
        Self { charm_repository }
    }

    /// Mark a charm as spent by its txid and vout
    /// [RJJ-S01] Updated: now requires both txid and vout
    pub async fn mark_charm_as_spent(&self, txid: &str, vout: i32) -> Result<(), CharmError> {
        self.charm_repository
            .mark_charm_as_spent(txid, vout)
            .await
            .map_err(|e| CharmError::ProcessingError(format!("Failed to mark charm as spent: {}", e)))
    }

    /// Mark multiple charms as spent in a batch (optimized for performance)
    pub async fn mark_charms_as_spent_batch(&self, txids: Vec<String>) -> Result<(), CharmError> {
        self.charm_repository
            .mark_charms_as_spent_batch(txids)
            .await
            .map_err(|e| CharmError::ProcessingError(format!("Failed to mark charms as spent in batch: {}", e)))
    }
}
