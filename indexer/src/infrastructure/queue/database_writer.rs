//! Database writer service for processing charm queue
//!
//! This module provides a dedicated background service that processes charm save requests
//! from the queue and writes them to the database without blocking the main processing thread.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{Instant, interval};

use crate::domain::services::CharmService;
use crate::infrastructure::queue::charm_queue::{CharmDataSaveRequest, CharmQueue, QueueError};
use crate::utils::logging;

/// Configuration for the database writer service
#[derive(Debug, Clone)]
pub struct DatabaseWriterConfig {
    /// Maximum number of items to process in a single batch
    pub batch_size: usize,
    /// Maximum time to wait before processing a partial batch
    pub batch_timeout: Duration,
    /// Number of retry attempts for failed operations
    pub max_retries: u32,
    /// Delay between retry attempts
    pub retry_delay: Duration,
    /// Interval for logging performance metrics
    pub metrics_interval: Duration,
}

impl Default for DatabaseWriterConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            batch_timeout: Duration::from_millis(200),
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
            metrics_interval: Duration::from_secs(30),
        }
    }
}

/// High-performance database writer service
pub struct DatabaseWriter {
    charm_service: Arc<CharmService>,
    queue: CharmQueue,
    config: DatabaseWriterConfig,
    receiver: Option<mpsc::UnboundedReceiver<CharmDataSaveRequest>>,
}

impl DatabaseWriter {
    /// Create a new database writer service
    pub fn new(
        charm_service: Arc<CharmService>,
        queue: CharmQueue,
        receiver: mpsc::UnboundedReceiver<CharmDataSaveRequest>,
        config: Option<DatabaseWriterConfig>,
    ) -> Self {
        Self {
            charm_service,
            queue,
            config: config.unwrap_or_default(),
            receiver: Some(receiver),
        }
    }

    /// Start the database writer service
    /// This runs indefinitely in the background, processing charm save requests
    pub async fn start(mut self) -> Result<(), QueueError> {
        let mut receiver = self.receiver.take().ok_or(QueueError::ChannelClosed)?;
        let mut metrics_timer = interval(self.config.metrics_interval);
        let mut batch_timer = interval(self.config.batch_timeout);
        let mut current_batch = Vec::new();
        let mut last_metrics_log = Instant::now();

        logging::log_info(&format!(
            "[DATABASE_WRITER] 🚀 Starting database writer service (batch_size: {}, timeout: {:?})",
            self.config.batch_size, self.config.batch_timeout
        ));

        // Optimize DB session for throughput (user-approved): disable synchronous_commit for this writer
        if let Err(e) = self.charm_service.optimize_writer_session().await {
            logging::log_warning(&format!(
                "[DATABASE_WRITER] ⚠️ Could not apply writer session optimization: {}",
                e
            ));
        } else {
            logging::log_info(
                "[DATABASE_WRITER] 🔧 Writer session optimized (synchronous_commit=off)",
            );
        }

        loop {
            tokio::select! {
                // Receive new charm save requests
                request = receiver.recv() => {
                    match request {
                        Some(req) => {
                            current_batch.push(req);

                            // Process batch if it reaches the configured size
                            if current_batch.len() >= self.config.batch_size {
                                self.process_batch(&mut current_batch).await;
                            }
                        }
                        None => {
                            // Channel closed, process remaining items and exit
                            if !current_batch.is_empty() {
                                self.process_batch(&mut current_batch).await;
                            }
                            logging::log_info("[DATABASE_WRITER] 🛑 Channel closed, shutting down");
                            break;
                        }
                    }
                }

                // Process partial batch on timeout
                _ = batch_timer.tick() => {
                    if !current_batch.is_empty() {
                        self.process_batch(&mut current_batch).await;
                    }
                }

                // Log performance metrics
                _ = metrics_timer.tick() => {
                    self.log_metrics(&last_metrics_log).await;
                    last_metrics_log = Instant::now();
                }
            }
        }

        Ok(())
    }

    /// Process a batch of charm data save requests
    async fn process_batch(&self, batch: &mut Vec<CharmDataSaveRequest>) {
        if batch.is_empty() {
            return;
        }

        let batch_size = batch.len();
        let start_time = Instant::now();

        logging::log_debug(&format!(
            "[DATABASE_WRITER] 📦 Processing batch of {} charms",
            batch_size
        ));

        // Convert requests to the format expected by save_batch
        // [RJJ-S01] Updated to use vout instead of charmid, added app_id and amount
        // [RJJ-DEX] Added tags field
        let charm_batch: Vec<(
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
            Option<String>,
        )> = batch
            .iter()
            .map(|req| {
                (
                    req.charm.txid.clone(),
                    req.charm.vout,
                    req.charm.block_height,
                    req.charm.data.clone(),
                    req.charm.asset_type.clone(),
                    req.charm.blockchain.clone(),
                    req.charm.network.clone(),
                    req.charm.address.clone(),
                    req.charm.app_id.clone(),
                    req.charm.amount,
                    req.charm.tags.clone(),
                )
            })
            .collect();

        // Convert transaction requests to batch format
        let transaction_batch: Vec<(
            String,
            u64,
            i64,
            serde_json::Value,
            serde_json::Value,
            i32,
            bool,
            String,
            String,
        )> = batch
            .iter()
            .map(|req| {
                let raw_json = serde_json::json!({
                    "hex": req.transaction.raw_hex,
                    "txid": req.transaction.txid
                });
                (
                    req.transaction.txid.clone(),
                    req.transaction.block_height,
                    req.transaction.tx_position,
                    raw_json,
                    req.charm.data.clone(), // Use charm data for transaction charm_data field
                    req.transaction.confirmations,
                    req.transaction.is_confirmed,
                    req.transaction.blockchain.clone(),
                    req.transaction.network.clone(),
                )
            })
            .collect();

        // Convert asset requests to batch format
        let asset_batch: Vec<(
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
        )> = batch
            .iter()
            .flat_map(|req| {
                req.assets.iter().map(|asset| {
                    (
                        asset.app_id.clone(),
                        req.charm.txid.clone(), // txid from charm
                        req.charm.vout,         // vout from charm
                        req.charm.block_height, // block_height from charm
                        asset.asset_type.clone(),
                        asset.supply,
                        asset.blockchain.clone(),
                        asset.network.clone(),
                        asset.name.clone(),
                        asset.symbol.clone(),
                        asset.description.clone(),
                        asset.image_url.clone(),
                        asset.decimals,
                    )
                })
            })
            .collect();

        // Process with retry logic
        let mut attempts = 0;
        let mut success = false;

        while attempts < self.config.max_retries && !success {
            attempts += 1;

            // Save all data in sequence: charms, transactions, then assets
            let mut all_success = true;
            let mut error_msg = String::new();

            // 1. Save charms first
            if let Err(e) = self.charm_service.save_batch(charm_batch.clone()).await {
                all_success = false;
                error_msg = format!("Failed to save charms: {}", e);
            }

            // 1b. Update stats_holders for new charms [RJJ-STATS-HOLDERS]
            // [RJJ-TOKEN-METADATA] Convert token app_ids to NFT app_ids for consolidation
            if all_success && !charm_batch.is_empty() {
                let holder_updates: Vec<(String, String, i64, i32)> = batch
                    .iter()
                    .filter_map(|req| {
                        req.charm.address.as_ref().map(|addr| {
                            // Convert token app_id (t/HASH) to NFT app_id (n/HASH) for consolidation
                            let nft_app_id = if req.charm.app_id.starts_with("t/") {
                                req.charm.app_id.replacen("t/", "n/", 1)
                            } else {
                                req.charm.app_id.clone()
                            };

                            (
                                nft_app_id,
                                addr.clone(),
                                req.charm.amount,
                                req.charm.block_height as i32,
                            )
                        })
                    })
                    .collect();

                if !holder_updates.is_empty() {
                    if let Err(e) = self
                        .charm_service
                        .get_stats_holders_repository()
                        .update_holders_batch(holder_updates)
                        .await
                    {
                        logging::log_warning(&format!(
                            "[DATABASE_WRITER] Failed to update stats_holders: {}",
                            e
                        ));
                        // Don't fail the entire batch for stats update failures
                    }
                }
            }

            // 2. Save transactions (only if charms succeeded)
            if all_success && !transaction_batch.is_empty() {
                if let Err(e) = self
                    .charm_service
                    .save_transaction_batch(transaction_batch.clone())
                    .await
                {
                    all_success = false;
                    error_msg = format!("Failed to save transactions: {}", e);
                }
            }

            // 3. Save assets (only if previous steps succeeded)
            if all_success && !asset_batch.is_empty() {
                if let Err(e) = self
                    .charm_service
                    .save_asset_batch(asset_batch.clone())
                    .await
                {
                    all_success = false;
                    error_msg = format!("Failed to save assets: {}", e);
                }
            }

            if all_success {
                success = true;

                // Mark all items as processed
                for _ in 0..batch_size {
                    self.queue.mark_processed();
                }

                let duration = start_time.elapsed();
                logging::log_debug(&format!(
                    "[DATABASE_WRITER] ✅ Successfully saved batch of {} charm data sets (charms + transactions + assets) in {:?} (attempt {})",
                    batch_size, duration, attempts
                ));

                // Warn if batch took too long
                if duration.as_secs_f64() > 2.0 {
                    logging::log_warning(&format!(
                        "[DATABASE_WRITER] 🐢 Slow batch: {} charm data sets took {:.2}s",
                        batch_size,
                        duration.as_secs_f64()
                    ));
                }
            } else {
                // Only log non-duplicate errors
                let is_duplicate_error =
                    error_msg.contains("duplicate key") || error_msg.contains("unique constraint");

                if !is_duplicate_error {
                    logging::log_warning(&format!(
                        "Failed to save charm data batch: {}",
                        error_msg
                    ));
                    logging::log_error(&format!(
                        "[DATABASE_WRITER] ❌ Failed to save batch (attempt {}/{}): {}",
                        attempts, self.config.max_retries, error_msg
                    ));
                } else {
                    // Duplicate errors are expected and not a problem - log as debug
                    logging::log_debug(&format!(
                        "[DATABASE_WRITER] ℹ️ Duplicate key detected (attempt {}), skipping",
                        attempts
                    ));
                }

                if attempts < self.config.max_retries {
                    tokio::time::sleep(self.config.retry_delay).await;
                } else {
                    // Mark all items as errors after max retries (only for non-duplicate errors)
                    if !is_duplicate_error {
                        for _ in 0..batch_size {
                            self.queue.mark_error();
                        }

                        logging::log_error(&format!(
                            "[DATABASE_WRITER] 💥 Failed to save batch after {} attempts, dropping {} charm data sets",
                            self.config.max_retries, batch_size
                        ));
                    } else {
                        // For duplicate errors, mark as processed since they're already in DB
                        for _ in 0..batch_size {
                            self.queue.mark_processed();
                        }
                    }
                }
            }
        }

        // Clear the batch
        batch.clear();
    }

    /// Log performance metrics
    async fn log_metrics(&self, last_log_time: &Instant) {
        let stats = self.queue.get_metrics();
        let elapsed = last_log_time.elapsed();

        let processing_rate = if elapsed.as_secs() > 0 {
            stats.total_processed as f64 / elapsed.as_secs() as f64
        } else {
            0.0
        };

        logging::log_info(&format!(
            "[DATABASE_WRITER] 📊 Queue Stats - Enqueued: {}, Processed: {}, Queue Size: {}, Errors: {}, Rate: {:.1}/sec",
            stats.total_enqueued,
            stats.total_processed,
            stats.current_queue_size,
            stats.processing_errors,
            processing_rate
        ));

        // Warn if queue is growing too large
        if stats.current_queue_size > 1000 {
            logging::log_warning(&format!(
                "[DATABASE_WRITER] ⚠️ Queue size is large ({}), consider increasing batch size or checking database performance",
                stats.current_queue_size
            ));
        }
    }
}

/// Builder for creating DatabaseWriter instances with custom configuration
pub struct DatabaseWriterBuilder {
    config: DatabaseWriterConfig,
}

impl DatabaseWriterBuilder {
    pub fn new() -> Self {
        Self {
            config: DatabaseWriterConfig::default(),
        }
    }

    pub fn batch_size(mut self, size: usize) -> Self {
        self.config.batch_size = size;
        self
    }

    pub fn batch_timeout(mut self, timeout: Duration) -> Self {
        self.config.batch_timeout = timeout;
        self
    }

    pub fn max_retries(mut self, retries: u32) -> Self {
        self.config.max_retries = retries;
        self
    }

    pub fn retry_delay(mut self, delay: Duration) -> Self {
        self.config.retry_delay = delay;
        self
    }

    pub fn metrics_interval(mut self, interval: Duration) -> Self {
        self.config.metrics_interval = interval;
        self
    }

    pub fn build(
        self,
        charm_service: Arc<CharmService>,
        queue: CharmQueue,
        receiver: mpsc::UnboundedReceiver<CharmDataSaveRequest>,
    ) -> DatabaseWriter {
        DatabaseWriter::new(charm_service, queue, receiver, Some(self.config))
    }
}

impl Default for DatabaseWriterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Tests removed temporarily due to complex mocking requirements
// TODO: Add comprehensive tests with proper mock setup
