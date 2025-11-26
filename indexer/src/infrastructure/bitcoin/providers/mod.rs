//! Bitcoin provider implementations
//! 
//! This module contains standardized provider implementations for different
//! Bitcoin data sources (QuickNode, Bitcoin Core nodes, etc.)

pub mod quicknode;
pub mod bitcoin_node;

pub use quicknode::QuickNodeProvider;
pub use bitcoin_node::BitcoinNodeProvider;

use crate::infrastructure::bitcoin::error::BitcoinClientError;
use bitcoincore_rpc::bitcoin::{Block, BlockHash};
use async_trait::async_trait;

/// Trait for Bitcoin providers (Bitcoin Core RPC, QuickNode, etc.)
#[async_trait]
pub trait BitcoinProvider: Send + Sync + std::fmt::Debug {
    /// Get the provider name for identification
    fn provider_name(&self) -> String;
    
    /// Get the current block count
    async fn get_block_count(&self) -> Result<u64, BitcoinClientError>;
    
    /// Get block hash by height
    async fn get_block_hash(&self, height: u64) -> Result<BlockHash, BitcoinClientError>;
    
    /// Get block by hash
    async fn get_block(&self, block_hash: &BlockHash) -> Result<Block, BitcoinClientError>;
    
    /// Get raw transaction hex
    async fn get_raw_transaction_hex(
        &self,
        txid: &str,
        block_hash: Option<&BlockHash>,
    ) -> Result<String, BitcoinClientError>;
    
    /// Apply provider-specific rate limiting
    async fn apply_rate_limiting(&self);
}
