//! Parallel transaction processor for high-throughput QuickNode calls
//! 
//! This module handles concurrent fetching and processing of Bitcoin transactions
//! to dramatically improve indexing speed for blocks with many transactions.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use bitcoincore_rpc::bitcoin::{Block, BlockHash};
use futures::stream::{FuturesUnordered, StreamExt};

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::BitcoinClient;

use super::batch_processor::{AssetBatchItem, CharmBatchItem, TransactionBatchItem};

/// Configuration for parallel transaction processing
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Maximum concurrent requests to QuickNode
    pub max_concurrent_requests: usize,
    /// Rate limit: requests per second
    pub requests_per_second: u64,
    /// Batch size for processing transactions
    pub batch_size: usize,
    /// Timeout for individual transaction requests
    pub request_timeout_ms: u64,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 128, // Aggressive concurrent requests
            requests_per_second: 500,     // Push network limits
            batch_size: 500,              // Much larger batches
            request_timeout_ms: 60000,    // Extended timeout for Bitcoin nodes
        }
    }
}

/// Result of processing a single transaction
#[derive(Debug)]
pub struct TransactionResult {
    pub charm_data: Option<(TransactionBatchItem, CharmBatchItem, Option<AssetBatchItem>)>,
    pub error: Option<String>,
}

/// High-performance parallel transaction processor
pub struct ParallelTransactionProcessor {
    bitcoin_client: Arc<BitcoinClient>,
    charm_service: Arc<CharmService>,
    config: ParallelConfig,
    rate_limiter: Arc<Semaphore>,
    #[allow(dead_code)]
    network_id: NetworkId,
}

impl ParallelTransactionProcessor {
    /// Create a new parallel transaction processor
    pub fn new(
        bitcoin_client: Arc<BitcoinClient>,
        charm_service: Arc<CharmService>,
        network_id: NetworkId,
        config: Option<ParallelConfig>,
    ) -> Self {
        let config = config.unwrap_or_default();
        let rate_limiter = Arc::new(Semaphore::new(config.max_concurrent_requests));
        

        Self {
            bitcoin_client,
            charm_service,
            config,
            rate_limiter,
            network_id,
        }
    }

    /// Process all transactions in a block in parallel batches
    pub async fn process_block_transactions(
        &self,
        block: &Block,
        block_hash: &BlockHash,
        height: u64,
        latest_height: u64,
        blockchain: &str,
        network: &str,
    ) -> Result<(Vec<TransactionBatchItem>, Vec<CharmBatchItem>, Vec<AssetBatchItem>), BlockProcessorError> {
        
        let _total_txs = block.txdata.len();
        

        let start_time = std::time::Instant::now();
        
        // Process transactions in batches
        let mut all_results = Vec::new();
        
        for (batch_num, batch) in block.txdata.chunks(self.config.batch_size).enumerate() {
            // Create tasks for this batch
            let batch_tasks: Vec<_> = batch
                .iter()
                .enumerate()
                .map(|(idx, tx)| {
                    let tx_pos = batch_num * self.config.batch_size + idx;
                    let txid = tx.txid().to_string();
                    self.process_transaction(
                        txid,
                        tx_pos,
                        block_hash.clone(),
                        height,
                        latest_height,
                        blockchain.to_string(),
                        network.to_string(),
                    )
                })
                .collect();

            // Execute batch tasks in parallel with concurrency limit
            let mut futures = FuturesUnordered::new();
            for task in batch_tasks {
                futures.push(task);
            }
            
            let mut batch_results = Vec::new();
            while let Some(result) = futures.next().await {
                batch_results.push(result);
            }
            all_results.extend(batch_results);


            // No delay between batches for maximum performance
        }

        let _processing_time = start_time.elapsed();
        
        // Collect results
        let mut transaction_batch = Vec::new();
        let mut charm_batch = Vec::new();
        let mut asset_batch = Vec::new();
        let mut _success_count = 0;
        let mut _error_count = 0;

        for result in all_results {
            match result {
                Ok(tx_result) => {
                    if let Some((tx_item, charm_item, asset_item)) = tx_result.charm_data {
                        transaction_batch.push(tx_item);
                        charm_batch.push(charm_item);
                        if let Some(asset) = asset_item {
                            asset_batch.push(asset);
                        }
                        _success_count += 1;
                    }
                    if tx_result.error.is_some() {
                        _error_count += 1;
                    }
                }
                Err(_) => {
                    _error_count += 1;
                }
            }
        }


        Ok((transaction_batch, charm_batch, asset_batch))
    }

    /// Process a single transaction with rate limiting and error handling
    async fn process_transaction(
        &self,
        txid: String,
        tx_pos: usize,
        block_hash: BlockHash,
        height: u64,
        latest_height: u64,
        _blockchain: String,
        _network: String,
    ) -> Result<TransactionResult, BlockProcessorError> {
        // Acquire semaphore permit for rate limiting
        let _permit = self.rate_limiter.acquire().await.map_err(|_| {
            BlockProcessorError::ProcessingError("Failed to acquire rate limit permit".to_string())
        })?;

        // Fetch raw transaction hex
        let raw_hex = match tokio::time::timeout(
            Duration::from_millis(self.config.request_timeout_ms),
            self.bitcoin_client.get_raw_transaction_hex(&txid, Some(&block_hash))
        ).await {
            Ok(Ok(hex)) => Some(hex),
            Ok(Err(e)) => {
                return Ok(TransactionResult {
                    charm_data: None,
                    error: Some(format!("Failed to get raw transaction: {}", e)),
                });
            }
            Err(_) => {
                return Ok(TransactionResult {
                    charm_data: None,
                    error: Some("Request timeout".to_string()),
                });
            }
        };

        // Process for charms asynchronously (non-blocking)
        if let Some(hex) = raw_hex.as_ref() {
            let charm_service = self.charm_service.clone();
            let txid_clone = txid.clone();
            let hex_clone = hex.clone();
            
            // Spawn charm detection in background - don't await it
            tokio::spawn(async move {
                let _ = charm_service
                    .detect_and_process_charm_with_raw_hex_and_latest(&txid_clone, height, &hex_clone, tx_pos, latest_height)
                    .await;
            });
        }

        // Return immediately without waiting for charm detection
        let charm_data = None;

        Ok(TransactionResult {
            charm_data,
            error: None,
        })
    }

    /// Get current performance statistics
    pub fn get_stats(&self) -> ParallelProcessorStats {
        ParallelProcessorStats {
            max_concurrent: self.config.max_concurrent_requests,
            rate_limit: self.config.requests_per_second,
            batch_size: self.config.batch_size,
            available_permits: self.rate_limiter.available_permits(),
        }
    }
}

/// Performance statistics for the parallel processor
#[derive(Debug)]
pub struct ParallelProcessorStats {
    pub max_concurrent: usize,
    pub rate_limit: u64,
    pub batch_size: usize,
    pub available_permits: usize,
}
