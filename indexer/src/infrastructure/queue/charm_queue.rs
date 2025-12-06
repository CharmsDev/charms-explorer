//! Asynchronous queue system for charm processing
//!
//! This module provides a high-performance, thread-safe queue that decouples
//! charm detection from database operations, preventing I/O bottlenecks.

use crate::domain::models::Charm;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Request to save a transaction to the database
#[derive(Debug, Clone)]
pub struct TransactionSaveRequest {
    pub txid: String,
    pub block_height: u64,
    pub tx_position: i64,
    pub raw_hex: String,
    pub confirmations: i32,
    pub is_confirmed: bool,
    pub blockchain: String,
    pub network: String,
}

/// Request to save an asset to the database
#[derive(Debug, Clone)]
pub struct AssetSaveRequest {
    pub app_id: String,
    pub asset_type: String,
    pub supply: u64,
    pub blockchain: String,
    pub network: String,
    // Metadata fields (optional, extracted from NFT)
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub decimals: Option<u8>,
}

/// Request to save a charm to the database
/// [RJJ-S01] Removed charmid field, added app_id and amount
#[derive(Debug, Clone)]
pub struct CharmSaveRequest {
    pub txid: String,
    pub vout: i32,
    pub block_height: u64,
    pub data: Value,
    pub asset_type: String,
    pub blockchain: String,
    pub network: String,
    pub address: Option<String>,
    pub tx_position: i64,
    pub app_id: String,
    pub amount: i64,
}

/// Unified request to save charm-related data (charm + transaction + assets)
#[derive(Debug, Clone)]
pub struct CharmDataSaveRequest {
    pub charm: CharmSaveRequest,
    pub transaction: TransactionSaveRequest,
    pub assets: Vec<AssetSaveRequest>,
}

impl CharmSaveRequest {
    /// Create a new charm save request from a detected charm
    /// [RJJ-S01] Updated to use vout instead of charmid, added app_id and amount
    pub fn from_charm(charm: &Charm, tx_position: i64) -> Self {
        Self {
            txid: charm.txid.clone(),
            vout: charm.vout,
            block_height: charm.block_height.unwrap_or(0),
            data: charm.data.clone(),
            asset_type: charm.asset_type.clone(),
            blockchain: charm.blockchain.clone(),
            network: charm.network.clone(),
            address: charm.address.clone(),
            tx_position,
            app_id: charm.app_id.clone(),
            amount: charm.amount,
        }
    }

    /// Convert to domain Charm model
    /// [RJJ-S01] Removed charmid parameter, added app_id and amount
    pub fn to_charm(&self) -> Charm {
        Charm::new(
            self.txid.clone(),
            self.vout,
            Some(self.block_height),
            self.data.clone(),
            chrono::Utc::now().naive_utc(),
            self.asset_type.clone(),
            self.blockchain.clone(),
            self.network.clone(),
            self.address.clone(),
            false, // New charms from queue are unspent by default
            self.app_id.clone(),
            self.amount,
        )
    }
}

impl CharmDataSaveRequest {
    /// Create a unified save request from charm detection data
    pub fn new(
        charm: &Charm,
        tx_position: i64,
        raw_hex: String,
        latest_height: u64,
        assets: Vec<AssetSaveRequest>,
    ) -> Self {
        let confirmations = (latest_height - charm.block_height.unwrap_or(0) + 1) as i32;
        let is_confirmed = confirmations >= 6;

        Self {
            charm: CharmSaveRequest::from_charm(charm, tx_position),
            transaction: TransactionSaveRequest {
                txid: charm.txid.clone(),
                block_height: charm.block_height.unwrap_or(0),
                tx_position,
                raw_hex,
                confirmations,
                is_confirmed,
                blockchain: charm.blockchain.clone(),
                network: charm.network.clone(),
            },
            assets,
        }
    }
}

/// High-performance asynchronous queue for charm processing
#[derive(Clone)]
pub struct CharmQueue {
    sender: mpsc::UnboundedSender<CharmDataSaveRequest>,
    metrics: Arc<QueueMetrics>,
}

/// Queue performance metrics
#[derive(Debug, Default)]
pub struct QueueMetrics {
    pub total_enqueued: std::sync::atomic::AtomicU64,
    pub total_processed: std::sync::atomic::AtomicU64,
    pub current_queue_size: std::sync::atomic::AtomicU64,
    pub processing_errors: std::sync::atomic::AtomicU64,
}

impl QueueMetrics {
    /// Get current queue statistics
    pub fn get_stats(&self) -> QueueStats {
        QueueStats {
            total_enqueued: self
                .total_enqueued
                .load(std::sync::atomic::Ordering::Relaxed),
            total_processed: self
                .total_processed
                .load(std::sync::atomic::Ordering::Relaxed),
            current_queue_size: self
                .current_queue_size
                .load(std::sync::atomic::Ordering::Relaxed),
            processing_errors: self
                .processing_errors
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub total_enqueued: u64,
    pub total_processed: u64,
    pub current_queue_size: u64,
    pub processing_errors: u64,
}

impl CharmQueue {
    /// Create a new charm queue with unbounded capacity
    /// Returns (queue, receiver) tuple
    pub fn new() -> (Self, mpsc::UnboundedReceiver<CharmDataSaveRequest>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let metrics = Arc::new(QueueMetrics::default());

        let queue = Self { sender, metrics };

        (queue, receiver)
    }

    /// Enqueue charm data (charm + transaction + assets) for asynchronous database saving
    /// This operation is non-blocking and returns immediately
    pub fn enqueue_charm_data(&self, request: CharmDataSaveRequest) -> Result<(), QueueError> {
        match self.sender.send(request) {
            Ok(_) => {
                self.metrics
                    .total_enqueued
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.metrics
                    .current_queue_size
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                crate::utils::logging::log_debug(&format!(
                    "[{}] 📤 Enqueued charm for async processing (queue size: {})",
                    "QUEUE",
                    self.metrics
                        .current_queue_size
                        .load(std::sync::atomic::Ordering::Relaxed)
                ));

                Ok(())
            }
            Err(_) => {
                self.metrics
                    .processing_errors
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Err(QueueError::ChannelClosed)
            }
        }
    }

    /// Get queue performance metrics
    pub fn get_metrics(&self) -> QueueStats {
        self.metrics.get_stats()
    }

    /// Check if the queue is healthy (sender is still connected)
    pub fn is_healthy(&self) -> bool {
        !self.sender.is_closed()
    }

    /// Mark an item as processed (called by database writer)
    pub fn mark_processed(&self) {
        self.metrics
            .total_processed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics
            .current_queue_size
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Mark a processing error (called by database writer)
    pub fn mark_error(&self) {
        self.metrics
            .processing_errors
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics
            .current_queue_size
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Queue-related errors
#[derive(Debug)]
pub enum QueueError {
    ChannelClosed,
    QueueFull,
    DatabaseError(String),
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueError::ChannelClosed => write!(f, "Queue channel is closed"),
            QueueError::QueueFull => write!(f, "Queue is full"),
            QueueError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for QueueError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_charm_queue_basic_operations() {
        let (queue, mut receiver) = CharmQueue::new();

        // Test enqueue
        let request = CharmSaveRequest {
            txid: "test_tx".to_string(),
            vout: 0,
            block_height: 100,
            data: json!({"test": "data"}),
            asset_type: "token".to_string(),
            blockchain: "bitcoin".to_string(),
            network: "mainnet".to_string(),
            address: Some("bc1test".to_string()),
            tx_position: 1,
            app_id: "t/test".to_string(),
            amount: 1000000000,
        };

        assert!(queue.enqueue_charm(request.clone()).is_ok());
        assert!(queue.is_healthy());

        // Test receive
        let received = receiver.recv().await.unwrap();
        assert_eq!(received.txid, "test_tx");
        assert_eq!(received.block_height, 100);

        // Test metrics
        let stats = queue.get_metrics();
        assert_eq!(stats.total_enqueued, 1);
    }

    #[test]
    fn test_charm_save_request_conversion() {
        // [RJJ-S01] Updated test to remove charmid, added app_id and amount
        let charm = Charm::new(
            "test_tx".to_string(),
            0, // vout
            100,
            json!({"test": "data"}),
            chrono::Utc::now().naive_utc(),
            "token".to_string(),
            "bitcoin".to_string(),
            "mainnet".to_string(),
            Some("bc1test".to_string()),
            false,
            "t/test".to_string(),
            1000000000,
        );

        let request = CharmSaveRequest::from_charm(&charm, 1);
        assert_eq!(request.txid, charm.txid);
        assert_eq!(request.vout, charm.vout);
        assert_eq!(request.block_height, charm.block_height);

        let converted_charm = request.to_charm();
        assert_eq!(converted_charm.txid, charm.txid);
        assert_eq!(converted_charm.vout, charm.vout);
    }
}
