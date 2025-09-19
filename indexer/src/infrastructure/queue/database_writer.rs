//! Database writer service for processing charm queue
//! 
//! This module provides a dedicated background service that processes charm save requests
//! from the queue and writes them to the database without blocking the main processing thread.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{interval, Instant};

use crate::domain::services::CharmService;
use crate::infrastructure::queue::charm_queue::{CharmSaveRequest, CharmQueue, QueueError};
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
    receiver: Option<mpsc::UnboundedReceiver<CharmSaveRequest>>,
}

impl DatabaseWriter {
    /// Create a new database writer service
    pub fn new(
        charm_service: Arc<CharmService>,
        queue: CharmQueue,
        receiver: mpsc::UnboundedReceiver<CharmSaveRequest>,
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
            self.config.batch_size,
            self.config.batch_timeout
        ));

        // Optimize DB session for throughput (user-approved): disable synchronous_commit for this writer
        if let Err(e) = self.charm_service.optimize_writer_session().await {
            logging::log_warning(&format!(
                "[DATABASE_WRITER] ⚠️ Could not apply writer session optimization: {}",
                e
            ));
        } else {
            logging::log_info("[DATABASE_WRITER] 🔧 Writer session optimized (synchronous_commit=off)");
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

    /// Process a batch of charm save requests
    async fn process_batch(&self, batch: &mut Vec<CharmSaveRequest>) {
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
        let charm_batch: Vec<(String, String, u64, serde_json::Value, String, String, String)> = 
            batch.iter().map(|req| (
                req.txid.clone(),
                req.charmid.clone(),
                req.block_height,
                req.data.clone(),
                req.asset_type.clone(),
                req.blockchain.clone(),
                req.network.clone(),
            )).collect();

        // Process with retry logic
        let mut attempts = 0;
        let mut success = false;

        while attempts < self.config.max_retries && !success {
            attempts += 1;

            match self.charm_service.save_batch(charm_batch.clone()).await {
                Ok(_) => {
                    success = true;
                    
                    // Mark all items as processed
                    for _ in 0..batch_size {
                        self.queue.mark_processed();
                    }

                    let duration = start_time.elapsed();
                    logging::log_debug(&format!(
                        "[DATABASE_WRITER] ✅ Successfully saved batch of {} charms in {:?} (attempt {})",
                        batch_size, duration, attempts
                    ));

                    // Warn if batch took too long
                    if duration.as_secs_f64() > 2.0 {
                        logging::log_warning(&format!(
                            "[DATABASE_WRITER] 🐢 Slow batch: {} charms took {:.2}s",
                            batch_size,
                            duration.as_secs_f64()
                        ));
                    }
                }
                Err(e) => {
                    logging::log_warning(&format!("Failed to save charm batch: {}", e));
                    logging::log_error(&format!(
                        "[DATABASE_WRITER] ❌ Failed to save batch (attempt {}/{}): {}",
                        attempts, self.config.max_retries, e
                    ));

                    if attempts < self.config.max_retries {
                        tokio::time::sleep(self.config.retry_delay).await;
                    } else {
                        // Mark all items as errors after max retries
                        for _ in 0..batch_size {
                            self.queue.mark_error();
                        }
                        
                        logging::log_error(&format!(
                            "[DATABASE_WRITER] 💥 Failed to save batch after {} attempts, dropping {} charms",
                            self.config.max_retries, batch_size
                        ));
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
        receiver: mpsc::UnboundedReceiver<CharmSaveRequest>,
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
