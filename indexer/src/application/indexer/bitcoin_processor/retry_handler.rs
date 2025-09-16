//! Retry handler for managing retry logic across operations

use std::future::Future;
use tokio::time::{sleep, Duration};

use crate::utils::logging;

/// Handles retry logic for operations that may fail temporarily
#[derive(Debug)]
pub struct RetryHandler {
    max_retries: u32,
    base_delay_ms: u64,
}

impl RetryHandler {
    pub fn new() -> Self {
        Self {
            max_retries: 5,
            base_delay_ms: 1000,
        }
    }

    pub fn with_config(max_retries: u32, base_delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms,
        }
    }

    /// Execute an operation with retry logic
    pub async fn execute_with_retry<F, Fut, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut retry_count = 0;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    retry_count += 1;

                    if retry_count >= self.max_retries {
                        return Err(e);
                    }

                    let delay = self.calculate_delay(retry_count);
                    logging::log_error(&format!(
                        "Operation failed (attempt {}/{}): {}. Retrying in {}ms",
                        retry_count, self.max_retries, e, delay
                    ));

                    sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }

    /// Execute an operation with retry logic and custom error handling
    pub async fn execute_with_retry_and_logging<F, Fut, T, E>(
        &self,
        operation: F,
        operation_name: &str,
        network_name: &str,
    ) -> Result<T, E>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut retry_count = 0;

        loop {
            match operation().await {
                Ok(result) => {
                    if retry_count > 0 {
                        logging::log_info(&format!(
                            "[{}] {} succeeded after {} retries",
                            network_name, operation_name, retry_count
                        ));
                    }
                    return Ok(result);
                }
                Err(e) => {
                    retry_count += 1;

                    if retry_count >= self.max_retries {
                        logging::log_error(&format!(
                            "[{}] {} failed after {} attempts: {}",
                            network_name, operation_name, self.max_retries, e
                        ));
                        return Err(e);
                    }

                    let delay = self.calculate_delay(retry_count);
                    logging::log_error(&format!(
                        "[{}] {} failed (attempt {}/{}): {}. Retrying in {}ms",
                        network_name, operation_name, retry_count, self.max_retries, e, delay
                    ));

                    sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }

    /// Calculate exponential backoff delay
    fn calculate_delay(&self, retry_count: u32) -> u64 {
        self.base_delay_ms * (2_u64.pow(retry_count.saturating_sub(1)))
    }
}

impl Default for RetryHandler {
    fn default() -> Self {
        Self::new()
    }
}
