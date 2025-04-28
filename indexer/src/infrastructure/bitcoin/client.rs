use bitcoincore_rpc::bitcoin::Block;
use bitcoincore_rpc::bitcoin::BlockHash;
use bitcoincore_rpc::bitcoin::Txid;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::str::FromStr;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::infrastructure::bitcoin::error::BitcoinClientError;

/// Client for interacting with the Bitcoin Core RPC API
pub struct BitcoinClient {
    client: Arc<Client>,
}

impl BitcoinClient {
    /// Create a new Bitcoin client
    pub fn new(config: &AppConfig) -> Result<Self, BitcoinClientError> {
        let rpc_url = format!("http://{}:{}", config.bitcoin.host, config.bitcoin.port);
        let auth = Auth::UserPass(
            config.bitcoin.username.clone(),
            config.bitcoin.password.clone(),
        );

        match Client::new(&rpc_url, auth) {
            Ok(client) => Ok(BitcoinClient {
                client: Arc::new(client),
            }),
            Err(e) => Err(BitcoinClientError::ConnectionError(format!(
                "Failed to connect to Bitcoin RPC: {}",
                e
            ))),
        }
    }

    /// Clone the client
    pub fn clone(&self) -> Self {
        BitcoinClient {
            client: self.client.clone(),
        }
    }

    /// Get the current block count
    pub fn get_block_count(&self) -> Result<u64, BitcoinClientError> {
        self.client.get_block_count().map_err(|e| e.into())
    }

    /// Get the block hash for a given height
    pub fn get_block_hash(&self, height: u64) -> Result<BlockHash, BitcoinClientError> {
        self.client.get_block_hash(height).map_err(|e| e.into())
    }

    /// Get the block for a given hash
    pub fn get_block(&self, hash: &BlockHash) -> Result<Block, BitcoinClientError> {
        self.client.get_block(hash).map_err(|e| e.into())
    }

    /// Get the raw transaction hex for a given transaction ID
    pub fn get_raw_transaction_hex(&self, txid: &str) -> Result<String, BitcoinClientError> {
        // Parse the transaction ID
        let txid_parsed = match Txid::from_str(txid) {
            Ok(txid) => txid,
            Err(e) => {
                return Err(BitcoinClientError::Other(format!(
                    "Invalid transaction ID: {}",
                    e
                )))
            }
        };

        // Get the raw transaction
        match self.client.get_raw_transaction(&txid_parsed, None) {
            Ok(tx) => {
                // Convert the transaction to hex
                let tx_bytes = bitcoincore_rpc::bitcoin::consensus::serialize(&tx);
                Ok(hex::encode(tx_bytes))
            }
            Err(e) => Err(BitcoinClientError::RpcError(e)),
        }
    }
}
