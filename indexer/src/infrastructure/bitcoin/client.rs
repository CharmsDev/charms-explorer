use bitcoincore_rpc::bitcoin::Block;
use bitcoincore_rpc::bitcoin::BlockHash;
use bitcoincore_rpc::bitcoin::Txid;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::str::FromStr;
use std::sync::Arc;

use crate::config::{AppConfig, BitcoinConfig, NetworkId, NetworkType};
use crate::infrastructure::bitcoin::error::BitcoinClientError;
use crate::utils::logging;

/// Provides access to Bitcoin Core RPC API
#[derive(Debug)]
pub struct BitcoinClient {
    client: Arc<Client>,
    network_id: NetworkId,
}

impl BitcoinClient {
    /// Creates a new Bitcoin client for a specific network
    pub fn new(bitcoin_config: &BitcoinConfig) -> Result<Self, BitcoinClientError> {
        let rpc_url = format!("http://{}:{}", bitcoin_config.host, bitcoin_config.port);
        let auth = Auth::UserPass(
            bitcoin_config.username.clone(),
            bitcoin_config.password.clone(),
        );

        let network_id = NetworkId::new(NetworkType::Bitcoin, &bitcoin_config.network);

        // Log connection details
        logging::log_bitcoin_connection_details(
            &bitcoin_config.host,
            &bitcoin_config.port,
            &bitcoin_config.username,
            &bitcoin_config.password,
            &bitcoin_config.network,
        );

        match Client::new(&rpc_url, auth) {
            Ok(client) => {
                logging::log_info(&format!(
                    "Successfully connected to Bitcoin RPC for network {}",
                    network_id.name
                ));
                Ok(BitcoinClient {
                    client: Arc::new(client),
                    network_id,
                })
            }
            Err(e) => {
                logging::log_error(&format!(
                    "Failed to connect to Bitcoin RPC for network {}: {}",
                    network_id.name, e
                ));
                Err(BitcoinClientError::ConnectionError(format!(
                    "Failed to connect to Bitcoin RPC for network {:?}: {}",
                    network_id, e
                )))
            }
        }
    }

    /// Creates a Bitcoin client from application configuration
    pub fn from_app_config(config: &AppConfig, network: &str) -> Result<Self, BitcoinClientError> {
        match config.get_bitcoin_config(network) {
            Some(bitcoin_config) => Self::new(bitcoin_config),
            None => Err(BitcoinClientError::ConfigError(format!(
                "Bitcoin configuration for network '{}' not found",
                network
            ))),
        }
    }

    /// Returns the network identifier for this client
    pub fn network_id(&self) -> &NetworkId {
        &self.network_id
    }

    /// Creates a new client instance with the same connection
    pub fn clone(&self) -> Self {
        BitcoinClient {
            client: self.client.clone(),
            network_id: self.network_id.clone(),
        }
    }

    /// Returns the current blockchain height
    pub fn get_block_count(&self) -> Result<u64, BitcoinClientError> {
        match self.client.get_block_count() {
            Ok(count) => Ok(count),
            Err(e) => Err(e.into()),
        }
    }

    /// Returns the block hash at specified height
    pub fn get_block_hash(&self, height: u64) -> Result<BlockHash, BitcoinClientError> {
        self.client.get_block_hash(height).map_err(|e| e.into())
    }

    /// Returns the best (tip) block hash
    pub fn get_best_block_hash(&self) -> Result<BlockHash, BitcoinClientError> {
        self.client.get_best_block_hash().map_err(|e| e.into())
    }

    /// Returns the full block data for specified hash
    pub fn get_block(&self, hash: &BlockHash) -> Result<Block, BitcoinClientError> {
        self.client.get_block(hash).map_err(|e| e.into())
    }

    /// Returns raw transaction hex, using block_hash for nodes without txindex
    pub fn get_raw_transaction_hex(
        &self,
        txid: &str,
        block_hash: Option<&BlockHash>,
    ) -> Result<String, BitcoinClientError> {
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

        // Get the raw transaction, passing the block hash if available
        match self.client.get_raw_transaction(&txid_parsed, block_hash) {
            Ok(tx) => {
                // Convert the transaction to hex
                let tx_bytes = bitcoincore_rpc::bitcoin::consensus::serialize(&tx);
                Ok(hex::encode(tx_bytes))
            }
            Err(e) => Err(BitcoinClientError::RpcError(e)),
        }
    }
}
