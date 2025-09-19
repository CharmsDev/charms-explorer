//! Charm queue service integration
//! 
//! This service provides a clean interface between charm detection and the async queue system.
//! It maintains backward compatibility while adding queue-based processing.

use std::sync::Arc;
use crate::domain::models::Charm;
use crate::domain::errors::CharmError;
use crate::infrastructure::queue::{CharmQueue, charm_queue::CharmSaveRequest};
use crate::infrastructure::persistence::repositories::CharmRepository;

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
    pub fn new_with_queue(
        charm_repository: Arc<CharmRepository>,
        queue: CharmQueue,
    ) -> Self {
        Self {
            charm_repository,
            queue: Some(queue),
            use_queue: true,
        }
    }

    /// Save a charm - uses queue if available, otherwise direct database access
    pub async fn save_charm(&self, charm: &Charm, tx_position: i64) -> Result<(), CharmError> {
        if self.use_queue {
            self.save_charm_async(charm, tx_position).await
        } else {
            self.save_charm_direct(charm).await
        }
    }

    /// Save charm using async queue (non-blocking)
    async fn save_charm_async(&self, charm: &Charm, tx_position: i64) -> Result<(), CharmError> {
        let queue = self.queue.as_ref().ok_or_else(|| {
            CharmError::ProcessingError("Queue not initialized".to_string())
        })?;

        let request = CharmSaveRequest::from_charm(charm, tx_position);
        
        // Use enqueue_charm - it's already non-blocking with unbounded channel
        queue.enqueue_charm(request).map_err(|e| {
            CharmError::ProcessingError(format!("Failed to enqueue charm: {}", e))
        })?;

        crate::utils::logging::log_debug(&format!(
            "[{}] 🚀 Charm {} enqueued for async processing",
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
    pub async fn save_batch(
        &self,
        charms: Vec<(String, String, u64, serde_json::Value, String, String, String)>,
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
    pub fn get_queue_metrics(&self) -> Option<crate::infrastructure::queue::charm_queue::QueueStats> {
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
    use serde_json::json;
    use std::sync::Arc;

    // Mock repository for testing
    struct MockCharmRepository;

    #[async_trait::async_trait]
    impl crate::infrastructure::persistence::repositories::CharmRepositoryTrait for MockCharmRepository {
        async fn save_charm(&self, _charm: &Charm) -> Result<(), crate::infrastructure::persistence::error::DbError> {
            Ok(())
        }

        async fn save_batch(&self, _charms: Vec<(String, String, u64, serde_json::Value, String, String, String)>) -> Result<(), crate::infrastructure::persistence::error::DbError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_direct_charm_service() {
        let repo = Arc::new(CharmRepository::new(/* mock connection */));
        let service = CharmQueueService::new_direct(repo);

        assert!(!service.is_using_queue());
        assert!(service.is_queue_healthy()); // Should be true when no queue
        assert!(service.get_queue_metrics().is_none());

        let charm = Charm::new(
            "test_tx".to_string(),
            "test_charm".to_string(),
            100,
            json!({"test": "data"}),
            chrono::Utc::now().naive_utc(),
            "token".to_string(),
            "bitcoin".to_string(),
            "mainnet".to_string(),
            Some("bc1test".to_string()),
        );

        // Should work without queue
        assert!(service.save_charm(&charm, 1).await.is_ok());
    }

    #[tokio::test]
    async fn test_queue_charm_service() {
        let repo = Arc::new(CharmRepository::new(/* mock connection */));
        let (queue, _receiver) = CharmQueue::new();
        let service = CharmQueueService::new_with_queue(repo, queue);

        assert!(service.is_using_queue());
        assert!(service.is_queue_healthy());
        assert!(service.get_queue_metrics().is_some());

        let charm = Charm::new(
            "test_tx".to_string(),
            "test_charm".to_string(),
            100,
            json!({"test": "data"}),
            chrono::Utc::now().naive_utc(),
            "token".to_string(),
            "bitcoin".to_string(),
            "mainnet".to_string(),
            Some("bc1test".to_string()),
        );

        // Should enqueue without blocking
        assert!(service.save_charm(&charm, 1).await.is_ok());

        let metrics = service.get_queue_metrics().unwrap();
        assert_eq!(metrics.total_enqueued, 1);
    }
}
