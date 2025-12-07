//! Charm queue service integration
//!
//! This service provides a clean interface between charm detection and the async queue system.
//! It maintains backward compatibility while adding queue-based processing.

use crate::domain::errors::CharmError;
use crate::domain::models::Charm;
use crate::infrastructure::persistence::repositories::CharmRepository;
use crate::infrastructure::queue::{
    charm_queue::{AssetSaveRequest, CharmDataSaveRequest},
    CharmQueue,
};
use std::sync::Arc;

/// Service that handles charm processing with optional queue integration
#[derive(Clone)]
pub struct CharmQueueService {
    charm_repository: Arc<CharmRepository>,
    queue: Option<CharmQueue>,
    use_queue: bool,
}

impl CharmQueueService {
    /// Create a new service with direct database access (no queue)
    pub fn new_direct(charm_repository: Arc<CharmRepository>) -> Self {
        Self {
            charm_repository,
            queue: None,
            use_queue: false,
        }
    }

    /// Create a new service with queue integration
    pub fn new_with_queue(charm_repository: Arc<CharmRepository>, queue: CharmQueue) -> Self {
        Self {
            charm_repository,
            queue: Some(queue),
            use_queue: true,
        }
    }

    /// Save charm data (charm + transaction + assets) - uses queue if available, otherwise direct database access
    pub async fn save_charm_data(
        &self,
        charm: &Charm,
        tx_position: i64,
        raw_hex: String,
        latest_height: u64,
        assets: Vec<AssetSaveRequest>,
    ) -> Result<(), CharmError> {
        if self.use_queue {
            self.save_charm_data_async(charm, tx_position, raw_hex, latest_height, assets)
                .await
        } else {
            self.save_charm_direct(charm).await
        }
    }

    /// Save a charm - uses queue if available, otherwise direct database access (legacy method)
    pub async fn save_charm(&self, charm: &Charm, tx_position: i64) -> Result<(), CharmError> {
        // For backward compatibility, call save_charm_data with empty assets and dummy values
        self.save_charm_data(
            charm,
            tx_position,
            String::new(),
            charm.block_height.unwrap_or(0),
            vec![],
        )
        .await
    }

    /// Save charm data using async queue (non-blocking)
    async fn save_charm_data_async(
        &self,
        charm: &Charm,
        tx_position: i64,
        raw_hex: String,
        latest_height: u64,
        assets: Vec<AssetSaveRequest>,
    ) -> Result<(), CharmError> {
        let queue = self
            .queue
            .as_ref()
            .ok_or_else(|| CharmError::ProcessingError("Queue not initialized".to_string()))?;

        let request = CharmDataSaveRequest::new(charm, tx_position, raw_hex, latest_height, assets);

        // Use enqueue_charm_data - it's already non-blocking with unbounded channel
        queue.enqueue_charm_data(request).map_err(|e| {
            CharmError::ProcessingError(format!("Failed to enqueue charm data: {}", e))
        })?;

        crate::utils::logging::log_debug(&format!(
            "[{}] 🚀 Charm data {} enqueued for async processing",
            charm.network, charm.txid
        ));

        Ok(())
    }

    /// Save charm directly to database (blocking)
    async fn save_charm_direct(&self, charm: &Charm) -> Result<(), CharmError> {
        self.charm_repository
            .save_charm(charm)
            .await
            .map_err(|e| CharmError::ProcessingError(format!("Failed to save charm: {}", e)))
    }

    /// Save multiple charms in batch (always uses direct database access)
    /// [RJJ-S01] Updated: replaced charmid with vout, added app_id and amount
    /// [RJJ-ADDRESS] Added address field
    pub async fn save_batch(
        &self,
        charms: Vec<(
            String,
            i32,
            u64,
            serde_json::Value,
            String,
            String,
            String,
            Option<String>,
            String,
            i64,
        )>,
    ) -> Result<(), CharmError> {
        self.charm_repository
            .save_batch(charms)
            .await
            .map_err(|e| CharmError::ProcessingError(format!("Failed to save charm batch: {}", e)))
    }

    /// Check if queue is being used
    pub fn is_using_queue(&self) -> bool {
        self.use_queue
    }

    /// Get queue metrics if available
    pub fn get_queue_metrics(
        &self,
    ) -> Option<crate::infrastructure::queue::charm_queue::QueueStats> {
        self.queue.as_ref().map(|q| q.get_metrics())
    }

    /// Check if queue is healthy
    pub fn is_queue_healthy(&self) -> bool {
        self.queue.as_ref().map_or(true, |q| q.is_healthy())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock repository for testing
    struct MockCharmRepository;

    #[async_trait::async_trait]
    impl crate::infrastructure::persistence::repositories::CharmRepositoryTrait
        for MockCharmRepository
    {
        async fn save_charm(
            &self,
            _charm: &Charm,
        ) -> Result<(), crate::infrastructure::persistence::error::DbError> {
            Ok(())
        }

        async fn save_batch(
            &self,
            _charms: Vec<(
                String,
                String,
                u64,
                serde_json::Value,
                String,
                String,
                String,
            )>,
        ) -> Result<(), crate::infrastructure::persistence::error::DbError> {
            Ok(())
        }
    }

    // Tests temporarily disabled due to mock setup complexity
    // TODO: Add proper mock database connection for testing
}
