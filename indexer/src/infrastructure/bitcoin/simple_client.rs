//! Simplified Bitcoin client using the new provider system

use bitcoincore_rpc::bitcoin::{Block, BlockHash};
use std::sync::Arc;

use crate::config::{BitcoinConfig, NetworkId, NetworkType};
use crate::infrastructure::bitcoin::error::BitcoinClientError;
use crate::infrastructure::bitcoin::provider_factory::ProviderFactory;
use crate::infrastructure::bitcoin::providers::BitcoinProvider;

/// Simple Bitcoin client that wraps a single provider
#[derive(Debug, Clone)]
pub struct SimpleBitcoinClient {
    provider: Arc<dyn BitcoinProvider>,
    network_id: NetworkId,
}

impl SimpleBitcoinClient {
    /// Create a new simple Bitcoin client from configuration
    pub fn new(config: &BitcoinConfig) -> Result<Self, BitcoinClientError> {
        let provider = ProviderFactory::create_provider(config)?;
        let network_id = NetworkId::new(NetworkType::Bitcoin, &config.network);

        crate::utils::logging::log_info(&format!("Using provider: {}", provider.provider_name()));

        Ok(Self {
            provider,
            network_id,
        })
    }

    /// Get the current block count
    pub async fn get_block_count(&self) -> Result<u64, BitcoinClientError> {
        self.provider.get_block_count().await
    }

    /// Get block hash by height
    pub async fn get_block_hash(&self, height: u64) -> Result<BlockHash, BitcoinClientError> {
        self.provider.get_block_hash(height).await
    }

    /// Get block by hash
    pub async fn get_block(&self, block_hash: &BlockHash) -> Result<Block, BitcoinClientError> {
        self.provider.get_block(block_hash).await
    }

    /// Get raw transaction hex
    pub async fn get_raw_transaction_hex(
        &self,
        txid: &str,
        block_hash: Option<&BlockHash>,
    ) -> Result<String, BitcoinClientError> {
        // Apply provider-specific rate limiting
        self.provider.apply_rate_limiting().await;

        self.provider
            .get_raw_transaction_hex(txid, block_hash)
            .await
    }

    /// Get the network ID
    pub fn network_id(&self) -> &NetworkId {
        &self.network_id
    }
}
