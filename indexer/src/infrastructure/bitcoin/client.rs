use bitcoincore_rpc::bitcoin::Block;
use bitcoincore_rpc::bitcoin::BlockHash;
use bitcoincore_rpc::bitcoin::Txid;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use crate::config::{BitcoinConfig, NetworkId, NetworkType};
use crate::infrastructure::bitcoin::SimpleBitcoinClient;
use crate::infrastructure::bitcoin::error::BitcoinClientError;
use crate::utils::logging;

/// Public Esplora gateway used to fill mempool propagation gaps for networks
/// where our local Bitcoin Core node trails the public P2P view (mainly
/// testnet4, where propagation is patchy). Disabled for mainnet by default.
fn supplement_mempool_url(network: &str) -> Option<String> {
    let env_key = format!("BITCOIN_{}_MEMPOOL_SUPPLEMENT_URL", network.to_uppercase());
    if let Ok(url) = env::var(&env_key) {
        return if url.trim().is_empty() { None } else { Some(url) };
    }
    match network {
        "testnet4" => Some("https://mempool.space/testnet4/api".to_string()),
        _ => None,
    }
}

/// Shared HTTP client for the supplement gateway. Built once so the connection
/// pool, TLS session cache and resolver cache survive across calls — the
/// previous per-request `Client::builder()` reopened a TCP+TLS handshake for
/// every txid we backfilled.
fn supplement_http() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .pool_idle_timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default()
    })
}

async fn fetch_esplora_mempool(base_url: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/mempool/txids", base_url.trim_end_matches('/'));
    let res = supplement_http()
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }
    res.json::<Vec<String>>().await.map_err(|e| e.to_string())
}

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

        logging::log_info(&format!(
            "Bitcoin RPC for {}: {}:{}",
            bitcoin_config.network, bitcoin_config.host, bitcoin_config.port
        ));

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
                .map_err(BitcoinClientError::RpcError)
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
                .map_err(BitcoinClientError::RpcError)
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

    /// Fetch all txids currently in the mempool. Starts from the local
    /// Bitcoin Core RPC (`getrawmempool`) and, for networks where the local
    /// node has incomplete P2P coverage (mainly testnet4), unions in the
    /// public Esplora gateway listing so propagation gaps don't hide
    /// pending spells from the explorer.
    ///
    /// The supplement also acts as a **fallback**, not just an augmentation: a
    /// local node that is down or refusing connections used to abort the whole
    /// call, which defeated the point of having a second source on exactly the
    /// network that needs it. Only an unusable local node *and* an unusable
    /// gateway is an error now.
    pub async fn get_raw_mempool(&self) -> Result<Vec<String>, BitcoinClientError> {
        let supplement = supplement_mempool_url(&self.network_id.name);

        let (mut txids, rpc_error) = if let Some(client) = &self.client {
            let client = client.clone();
            let rpc = tokio::task::spawn_blocking(move || {
                use bitcoincore_rpc::RpcApi;
                client
                    .get_raw_mempool()
                    .map(|t| t.iter().map(|x| x.to_string()).collect::<Vec<_>>())
                    .map_err(BitcoinClientError::RpcError)
            })
            .await
            .map_err(|e| BitcoinClientError::Other(format!("spawn_blocking join error: {}", e)))?;

            match rpc {
                Ok(txids) => (txids, None),
                // Hold the error: if a supplement is configured it may still
                // give us a usable view, so don't bail out before trying it.
                Err(e) if supplement.is_some() => (Vec::new(), Some(e)),
                Err(e) => return Err(e),
            }
        } else {
            // External provider clients are block-only; the supplement below
            // is the only mempool source we'd have.
            (Vec::new(), None)
        };

        if let Some(url) = supplement {
            match fetch_esplora_mempool(&url).await {
                Ok(extra) => {
                    if let Some(e) = &rpc_error {
                        logging::log_warning(&format!(
                            "[{}] getrawmempool failed ({}), serving {} txids from supplement {}",
                            self.network_id.name,
                            e,
                            extra.len(),
                            url
                        ));
                    }
                    // Track seen txids as we push so duplicates *within* the
                    // gateway response are also collapsed — the previous
                    // snapshot-before-loop set only deduped against the RPC view.
                    let mut seen: std::collections::HashSet<String> =
                        txids.iter().cloned().collect();
                    for t in extra {
                        if seen.insert(t.clone()) {
                            txids.push(t);
                        }
                    }
                }
                Err(e) => {
                    // Both sources unusable — surface the RPC error, which is
                    // the more actionable of the two.
                    if let Some(rpc_e) = rpc_error {
                        return Err(rpc_e);
                    }
                    logging::log_debug(&format!(
                        "[{}] mempool supplement {} failed: {}",
                        self.network_id.name, url, e
                    ));
                }
            }
        }

        Ok(txids)
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
            // bitcoincore_rpc is a blocking client — calling it directly here
            // stalled a tokio worker thread for the whole RPC roundtrip, once
            // per mempool txid. Offload it like get_raw_mempool already does.
            let client = client.clone();
            let block_hash = block_hash.copied();
            let rpc_result = tokio::task::spawn_blocking(move || {
                client
                    .get_raw_transaction(&txid_parsed, block_hash.as_ref())
                    .map(|tx| hex::encode(bitcoincore_rpc::bitcoin::consensus::serialize(&tx)))
            })
            .await
            .map_err(|e| BitcoinClientError::Other(format!("spawn_blocking join error: {}", e)))?;

            match rpc_result {
                Ok(hex) => Ok(hex),
                Err(e) => {
                    // Local node may not have the tx in its mempool when we
                    // discovered it via the Esplora supplement. Try the
                    // supplement gateway before giving up.
                    if let Some(url) = supplement_mempool_url(&self.network_id.name) {
                        if let Ok(hex) = fetch_esplora_tx_hex(&url, txid).await {
                            return Ok(hex);
                        }
                    }
                    Err(BitcoinClientError::RpcError(e))
                }
            }
        } else {
            Err(BitcoinClientError::ConnectionError(
                "No client available".to_string(),
            ))
        }
    }
}

async fn fetch_esplora_tx_hex(base_url: &str, txid: &str) -> Result<String, String> {
    let url = format!("{}/tx/{}/hex", base_url.trim_end_matches('/'), txid);
    let res = supplement_http()
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }
    res.text()
        .await
        .map(|t| t.trim().to_string())
        .map_err(|e| e.to_string())
}
