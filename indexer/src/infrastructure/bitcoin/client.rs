use bitcoincore_rpc::bitcoin::Block;
use bitcoincore_rpc::bitcoin::BlockHash;
use bitcoincore_rpc::bitcoin::Txid;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::str::FromStr;
use std::sync::Arc;

use crate::config::{AppConfig, BitcoinConfig, NetworkId, NetworkType};
use crate::infrastructure::bitcoin::SimpleBitcoinClient;
use crate::infrastructure::bitcoin::error::BitcoinClientError;
use crate::utils::logging;

/// Provides access to Bitcoin Core RPC API
#[derive(Debug, Clone)]
pub struct BitcoinClient {
    client: Option<Arc<Client>>,                // Legacy single client
    simple_client: Option<SimpleBitcoinClient>, // New simple client
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
                    client: Some(Arc::new(client)),
                    simple_client: None,
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

    /// Creates a Bitcoin client from a SimpleBitcoinClient
    pub fn from_simple_client(simple_client: SimpleBitcoinClient) -> Self {
        let network_id = simple_client.network_id().clone();
        BitcoinClient {
            client: None,
            simple_client: Some(simple_client),
            network_id,
        }
    }

    /// Returns the network identifier for this client
    pub fn network_id(&self) -> &NetworkId {
        &self.network_id
    }

    /// Returns the current blockchain height
    pub async fn get_block_count(&self) -> Result<u64, BitcoinClientError> {
        if let Some(simple_client) = &self.simple_client {
            simple_client.get_block_count().await
        } else if let Some(client) = &self.client {
            match client.get_block_count() {
                Ok(count) => Ok(count),
                Err(e) => Err(e.into()),
            }
        } else {
            Err(BitcoinClientError::ConnectionError(
                "No client available".to_string(),
            ))
        }
    }

    /// Returns the block hash at specified height
    pub async fn get_block_hash(&self, height: u64) -> Result<BlockHash, BitcoinClientError> {
        if let Some(simple_client) = &self.simple_client {
            let bitcoin_hash = simple_client.get_block_hash(height).await?;
            // Convert from bitcoin::BlockHash to bitcoincore_rpc::bitcoin::BlockHash
            bitcoincore_rpc::bitcoin::BlockHash::from_str(&bitcoin_hash.to_string()).map_err(|e| {
                BitcoinClientError::Other(format!("Failed to convert block hash: {}", e))
            })
        } else if let Some(client) = &self.client {
            client
                .get_block_hash(height)
                .map_err(|e| BitcoinClientError::RpcError(e))
        } else {
            Err(BitcoinClientError::ConnectionError(
                "No client available".to_string(),
            ))
        }
    }

    /// Returns the best (tip) block hash
    pub async fn get_best_block_hash(&self) -> Result<BlockHash, BitcoinClientError> {
        if let Some(simple_client) = &self.simple_client {
            // Get current block count and then get hash for that height
            let block_count = simple_client.get_block_count().await?;
            let bitcoin_hash = simple_client.get_block_hash(block_count).await?;
            // Convert from bitcoin::BlockHash to bitcoincore_rpc::bitcoin::BlockHash
            bitcoincore_rpc::bitcoin::BlockHash::from_str(&bitcoin_hash.to_string()).map_err(|e| {
                BitcoinClientError::Other(format!("Failed to convert best block hash: {}", e))
            })
        } else if let Some(client) = &self.client {
            client
                .get_best_block_hash()
                .map_err(|e| BitcoinClientError::RpcError(e))
        } else {
            Err(BitcoinClientError::ConnectionError(
                "No client available".to_string(),
            ))
        }
    }

    /// Returns the full block data for specified hash
    pub async fn get_block(&self, hash: &BlockHash) -> Result<Block, BitcoinClientError> {
        if let Some(simple_client) = &self.simple_client {
            simple_client.get_block(hash).await
        } else if let Some(client) = &self.client {
            client.get_block(hash).map_err(|e| e.into())
        } else {
            Err(BitcoinClientError::ConnectionError(
                "No client available".to_string(),
            ))
        }
    }

    /// Get the name of the currently active provider
    pub fn get_primary_provider_name(&self) -> Option<String> {
        if let Some(simple_client) = &self.simple_client {
            Some(simple_client.provider_name())
        } else {
            Some("BitcoinCore".to_string())
        }
    }

    /// Heuristic to determine if we're likely using a local Bitcoin node
    pub fn is_using_local_node(&self) -> bool {
        if let Some(simple_client) = &self.simple_client {
            simple_client.uses_local_node()
        } else {
            // Legacy single client - assume it's local
            true
        }
    }

    /// Check if external providers exist
    pub fn has_external_providers(&self) -> bool {
        if let Some(simple_client) = &self.simple_client {
            simple_client.is_external_provider()
        } else {
            false
        }
    }

    /// Returns the underlying RPC client Arc for direct RPC calls (e.g. getrawmempool)
    /// Only available when using the legacy single-client mode (not SimpleBitcoinClient)
    pub fn get_rpc_client(&self) -> Option<std::sync::Arc<bitcoincore_rpc::Client>> {
        self.client.clone()
    }

    /// Fetch all txids currently in the mempool via getrawmempool RPC.
    /// Returns an empty vec if the client doesn't support it (e.g. external providers).
    pub async fn get_raw_mempool(&self) -> Result<Vec<String>, BitcoinClientError> {
        if let Some(client) = &self.client {
            let client = client.clone();
            tokio::task::spawn_blocking(move || {
                use bitcoincore_rpc::RpcApi;
                client
                    .get_raw_mempool()
                    .map(|txids| txids.iter().map(|t| t.to_string()).collect::<Vec<_>>())
                    .map_err(|e| BitcoinClientError::RpcError(e))
            })
            .await
            .map_err(|e| BitcoinClientError::Other(format!("spawn_blocking join error: {}", e)))?
        } else {
            // SimpleBitcoinClient (external provider) — mempool polling not supported
            Err(BitcoinClientError::Other(
                "getrawmempool not available for external providers".to_string(),
            ))
        }
    }

    /// Returns raw transaction hex, using block_hash for nodes without txindex
    pub async fn get_raw_transaction_hex(
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
                )));
            }
        };

        if let Some(simple_client) = &self.simple_client {
            simple_client
                .get_raw_transaction_hex(txid, block_hash)
                .await
        } else if let Some(client) = &self.client {
            match client.get_raw_transaction(&txid_parsed, block_hash) {
                Ok(tx) => {
                    // Convert the transaction to hex
                    let tx_bytes = bitcoincore_rpc::bitcoin::consensus::serialize(&tx);
                    Ok(hex::encode(tx_bytes))
                }
                Err(e) => Err(BitcoinClientError::RpcError(e)),
            }
        } else {
            Err(BitcoinClientError::ConnectionError(
                "No client available".to_string(),
            ))
        }
    }
}
