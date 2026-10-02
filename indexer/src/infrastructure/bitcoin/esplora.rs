//! Esplora REST client (mempool.space by default).
//!
//! Single source of chain data for the indexer: tip, block hashes, raw
//! blocks and raw transactions. Requests are spaced by a small minimum gap
//! and retried on 429/5xx so a catch-up burst stays within the public
//! gateway's fair-use limits.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

use crate::infrastructure::bitcoin::error::BitcoinClientError;

/// Minimum gap between two requests to the gateway.
const MIN_REQUEST_GAP: Duration = Duration::from_millis(150);
/// Attempts per request before giving up (429/5xx/transport errors).
const MAX_ATTEMPTS: u32 = 5;

/// Default Esplora REST base for a network.
pub fn default_esplora_url(network: &str) -> String {
    match network {
        "mainnet" => "https://mempool.space/api".to_string(),
        other => format!("https://mempool.space/{}/api", other),
    }
}

/// Default mempool.space websocket for a network.
pub fn default_ws_url(network: &str) -> String {
    match network {
        "mainnet" => "wss://mempool.space/api/v1/ws".to_string(),
        other => format!("wss://mempool.space/{}/api/v1/ws", other),
    }
}

#[derive(Debug, Clone)]
pub struct EsploraClient {
    base_url: String,
    http: reqwest::Client,
    last_request: Arc<Mutex<Instant>>,
}

impl EsploraClient {
    pub fn new(base_url: &str) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(60))
            .user_agent("charms-explorer-indexer")
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
            last_request: Arc::new(Mutex::new(Instant::now() - MIN_REQUEST_GAP)),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn throttle(&self) {
        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed();
        if elapsed < MIN_REQUEST_GAP {
            tokio::time::sleep(MIN_REQUEST_GAP - elapsed).await;
        }
        *last = Instant::now();
    }

    /// GET with throttling and retry. Returns `Ok(None)` on 404.
    async fn get(&self, path: &str) -> Result<Option<reqwest::Response>, BitcoinClientError> {
        let url = format!("{}{}", self.base_url, path);
        let mut attempt = 0;
        loop {
            attempt += 1;
            self.throttle().await;
            let err = match self.http.get(&url).send().await {
                Ok(res) if res.status().is_success() => return Ok(Some(res)),
                Ok(res) if res.status() == reqwest::StatusCode::NOT_FOUND => return Ok(None),
                Ok(res) => {
                    let status = res.status();
                    let retryable = status == reqwest::StatusCode::TOO_MANY_REQUESTS
                        || status.is_server_error();
                    let msg = format!("GET {} -> HTTP {}", path, status);
                    if !retryable {
                        return Err(BitcoinClientError::NetworkError(msg));
                    }
                    msg
                }
                Err(e) => format!("GET {} failed: {}", path, e),
            };
            if attempt >= MAX_ATTEMPTS {
                return Err(BitcoinClientError::NetworkError(err));
            }
            tokio::time::sleep(Duration::from_millis(500 * 2u64.pow(attempt - 1))).await;
        }
    }

    async fn get_text(&self, path: &str) -> Result<String, BitcoinClientError> {
        let res = self.get(path).await?.ok_or_else(|| {
            BitcoinClientError::NetworkError(format!("GET {} -> not found", path))
        })?;
        res.text()
            .await
            .map(|t| t.trim().to_string())
            .map_err(|e| BitcoinClientError::NetworkError(e.to_string()))
    }

    pub async fn tip_height(&self) -> Result<u64, BitcoinClientError> {
        self.get_text("/blocks/tip/height")
            .await?
            .parse()
            .map_err(|e| BitcoinClientError::ParseError(format!("tip height: {}", e)))
    }

    pub async fn tip_hash(&self) -> Result<String, BitcoinClientError> {
        self.get_text("/blocks/tip/hash").await
    }

    pub async fn block_hash(&self, height: u64) -> Result<String, BitcoinClientError> {
        self.get_text(&format!("/block-height/{}", height)).await
    }

    pub async fn block_raw(&self, hash: &str) -> Result<Vec<u8>, BitcoinClientError> {
        let path = format!("/block/{}/raw", hash);
        let res = self.get(&path).await?.ok_or_else(|| {
            BitcoinClientError::NetworkError(format!("GET {} -> not found", path))
        })?;
        res.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| BitcoinClientError::NetworkError(e.to_string()))
    }

    pub async fn tx_hex(&self, txid: &str) -> Result<String, BitcoinClientError> {
        self.get_text(&format!("/tx/{}/hex", txid)).await
    }

    /// Whether the gateway still knows the tx (in mempool or confirmed).
    /// `Ok(false)` only on a definite 404.
    pub async fn tx_exists(&self, txid: &str) -> Result<bool, BitcoinClientError> {
        Ok(self.get(&format!("/tx/{}/status", txid)).await?.is_some())
    }
}
