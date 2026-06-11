//! Spent UTXO tracking for charms
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

    /// Mark multiple charms as spent in a batch, scoped by `network`.
    pub async fn mark_charms_as_spent_batch(
        &self,
        txid_vouts: Vec<(String, i32)>,
        network: &str,
    ) -> Result<(), CharmError> {
        self.charm_repository
            .mark_charms_as_spent_batch(txid_vouts, network)
            .await
            .map_err(|e| {
                CharmError::ProcessingError(format!(
                    "Failed to mark charms as spent in batch: {}",
                    e
                ))
            })
    }
}
