use async_trait::async_trait;
use std::fmt::Debug;
use tokio_util::sync::CancellationToken;

use crate::config::NetworkId;
use crate::domain::errors::BlockProcessorError;

/// Defines common interface for all blockchain processors
#[async_trait]
pub trait BlockchainProcessor: Send + Sync + Debug {
    /// Returns the network identifier for this processor
    fn network_id(&self) -> &NetworkId;

    /// Starts continuous block processing loop. The processor must exit
    /// cleanly when `cancel` fires (e.g. during graceful shutdown).
    async fn start_processing(
        &mut self,
        cancel: CancellationToken,
    ) -> Result<(), BlockProcessorError>;

    /// Processes a single block at specified height
    async fn process_block(&self, height: u64) -> Result<(), BlockProcessorError>;
}
