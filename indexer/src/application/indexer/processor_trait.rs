use async_trait::async_trait;
use std::fmt::Debug;

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;

/// Defines common interface for all blockchain processors
#[async_trait]
pub trait BlockchainProcessor: Send + Sync + Debug {
    /// Returns the network identifier for this processor
    fn network_id(&self) -> &NetworkId;

    /// Starts continuous block processing loop
    async fn start_processing(&mut self) -> Result<(), BlockProcessorError>;

    /// Processes a single block at specified height
    async fn process_block(&self, height: u64) -> Result<(), BlockProcessorError>;
}
