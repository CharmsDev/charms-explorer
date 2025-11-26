//! Bitcoin Core node provider implementation

use async_trait::async_trait;
use bitcoincore_rpc::bitcoin::{Block, BlockHash};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::sync::Arc;
use std::str::FromStr;

use crate::infrastructure::bitcoin::error::BitcoinClientError;
use super::BitcoinProvider;

/// Bitcoin node provider for direct RPC calls
#[derive(Debug)]
pub struct BitcoinNodeProvider {
    client: Arc<Client>,
    network: String,
}

impl BitcoinNodeProvider {
    /// Create a new Bitcoin node provider
    pub fn new(
        host: String,
        port: String,
        username: String,
        password: String,
        network: String,
    ) -> Result<Self, BitcoinClientError> {
        let url = format!("http://{}:{}", host, port);
        let auth = Auth::UserPass(username, password);
        
        let client = Client::new(&url, auth)
            .map_err(|e| BitcoinClientError::ConnectionError(e.to_string()))?;
        
        Ok(Self {
            client: Arc::new(client),
            network,
        })
    }
}

#[async_trait]
impl BitcoinProvider for BitcoinNodeProvider {
    fn provider_name(&self) -> String {
        format!("Bitcoin Node ({})", self.network)
    }

    async fn get_block_count(&self) -> Result<u64, BitcoinClientError> {
        let client = self.client.clone();
        tokio::task::spawn_blocking(move || {
            client.get_block_count()
                .map_err(|e| BitcoinClientError::RpcError(e))
        })
        .await
        .map_err(|e| BitcoinClientError::NetworkError(e.to_string()))?
    }

    async fn get_block_hash(&self, height: u64) -> Result<BlockHash, BitcoinClientError> {
        let client = self.client.clone();
        tokio::task::spawn_blocking(move || {
            client.get_block_hash(height)
                .map_err(|e| BitcoinClientError::RpcError(e))
        })
        .await
        .map_err(|e| BitcoinClientError::NetworkError(e.to_string()))?
    }

    async fn get_block(&self, block_hash: &BlockHash) -> Result<Block, BitcoinClientError> {
        let client = self.client.clone();
        let block_hash = *block_hash;
        tokio::task::spawn_blocking(move || {
            client.get_block(&block_hash)
                .map_err(|e| BitcoinClientError::RpcError(e))
        })
        .await
        .map_err(|e| BitcoinClientError::NetworkError(e.to_string()))?
    }

    async fn get_raw_transaction_hex(
        &self,
        txid: &str,
        block_hash: Option<&BlockHash>,
    ) -> Result<String, BitcoinClientError> {
        let client = self.client.clone();
        let txid = txid.to_string();
        let block_hash = block_hash.copied();
        
        tokio::task::spawn_blocking(move || {
            let txid = bitcoincore_rpc::bitcoin::Txid::from_str(&txid)
                .map_err(|e| BitcoinClientError::ParseError(e.to_string()))?;
            
            client.get_raw_transaction_hex(&txid, block_hash.as_ref())
                .map_err(|e| BitcoinClientError::RpcError(e))
        })
        .await
        .map_err(|e| BitcoinClientError::NetworkError(e.to_string()))?
    }

    async fn apply_rate_limiting(&self) {
        // Bitcoin nodes: No rate limiting - run at maximum capacity
    }
}
