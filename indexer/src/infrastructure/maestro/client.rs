//! Maestro client — minimal surface for the BTC auto-seeder.

use serde_json::Value;
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://xbt-mainnet.gomaestro-api.org/v0";
const TXS_PAGE_SIZE_HINT: usize = 25; // esplora returns up to 25 per page
const MAX_TXS_PAGES: usize = 10;

#[derive(Debug, Clone)]
pub struct MaestroUtxo {
    pub txid: String,
    pub vout: u32,
    pub value: u64,
    pub block_height: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct MaestroAddressTx {
    pub txid: String,
    pub direction: String, // "in" or "out" from this address's perspective
    pub amount: i64,
    pub fee: i64,
    pub block_height: Option<i32>,
    pub block_time: Option<i64>,
    pub confirmed: bool,
}

#[derive(Debug, Clone)]
pub struct MaestroChainTip {
    pub height: u64,
    pub hash: String,
}

#[derive(Debug)]
pub enum MaestroError {
    Http(String),
    Parse(String),
    Api { status: u16, body: String },
}

impl std::fmt::Display for MaestroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaestroError::Http(e) => write!(f, "http error: {}", e),
            MaestroError::Parse(e) => write!(f, "parse error: {}", e),
            MaestroError::Api { status, body } => write!(f, "api error {}: {}", status, body),
        }
    }
}

impl std::error::Error for MaestroError {}

#[derive(Clone)]
pub struct MaestroClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl MaestroClient {
    pub fn new(api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client build");
        Self {
            http,
            api_key,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    #[cfg(test)]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub async fn get_utxos(&self, address: &str) -> Result<Vec<MaestroUtxo>, MaestroError> {
        let resp = self
            .http
            .get(self.url(&format!("/esplora/address/{}/utxo", address)))
            .header("api-key", &self.api_key)
            .send()
            .await
            .map_err(|e| MaestroError::Http(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(MaestroError::Api {
                status: status.as_u16(),
                body,
            });
        }

        let raw: Vec<Value> = resp
            .json()
            .await
            .map_err(|e| MaestroError::Parse(e.to_string()))?;

        Ok(raw
            .iter()
            .filter_map(|u| {
                let confirmed = u["status"]["confirmed"].as_bool().unwrap_or(false);
                let block_height = if confirmed {
                    u["status"]["block_height"].as_i64().map(|h| h as i32)
                } else {
                    None
                };
                Some(MaestroUtxo {
                    txid: u["txid"].as_str()?.to_string(),
                    vout: u["vout"].as_u64()? as u32,
                    value: u["value"].as_u64()?,
                    block_height,
                })
            })
            .collect())
    }

    /// Paginated address tx history. Stops at MAX_TXS_PAGES (~250 txs) to
    /// bound seed time per address; charm holders rarely have more recent
    /// activity than that, and older history isn't critical for balance.
    pub async fn get_address_txs(
        &self,
        address: &str,
    ) -> Result<Vec<MaestroAddressTx>, MaestroError> {
        let mut all: Vec<MaestroAddressTx> = Vec::new();
        let mut after_txid: Option<String> = None;

        for _ in 0..MAX_TXS_PAGES {
            let path = match &after_txid {
                Some(txid) => format!("/esplora/address/{}/txs/chain/{}", address, txid),
                None => format!("/esplora/address/{}/txs", address),
            };
            let resp = self
                .http
                .get(self.url(&path))
                .header("api-key", &self.api_key)
                .send()
                .await
                .map_err(|e| MaestroError::Http(e.to_string()))?;

            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(MaestroError::Api {
                    status: status.as_u16(),
                    body,
                });
            }

            let txs: Vec<Value> = resp
                .json()
                .await
                .map_err(|e| MaestroError::Parse(e.to_string()))?;

            if txs.is_empty() {
                break;
            }

            for tx in &txs {
                if let Some(parsed) = parse_address_tx(tx, address) {
                    all.push(parsed);
                }
            }

            if txs.len() < TXS_PAGE_SIZE_HINT {
                break;
            }
            after_txid = txs
                .last()
                .and_then(|t| t["txid"].as_str().map(|s| s.to_string()));
        }

        Ok(all)
    }

    pub async fn get_chain_tip(&self) -> Result<MaestroChainTip, MaestroError> {
        let resp = self
            .http
            .get(self.url("/esplora/blocks/tip/height"))
            .header("api-key", &self.api_key)
            .send()
            .await
            .map_err(|e| MaestroError::Http(e.to_string()))?;
        let height: u64 = resp
            .text()
            .await
            .map_err(|e| MaestroError::Parse(e.to_string()))?
            .trim()
            .parse()
            .map_err(|e: std::num::ParseIntError| MaestroError::Parse(e.to_string()))?;

        let resp = self
            .http
            .get(self.url("/esplora/blocks/tip/hash"))
            .header("api-key", &self.api_key)
            .send()
            .await
            .map_err(|e| MaestroError::Http(e.to_string()))?;
        let hash = resp
            .text()
            .await
            .map_err(|e| MaestroError::Parse(e.to_string()))?
            .trim()
            .to_string();

        Ok(MaestroChainTip { height, hash })
    }
}

fn parse_address_tx(tx: &Value, address: &str) -> Option<MaestroAddressTx> {
    let txid = tx["txid"].as_str()?.to_string();
    let status = &tx["status"];
    let block_height = status["block_height"].as_i64().map(|h| h as i32);
    let block_time = status["block_time"].as_i64();
    let confirmed = status["confirmed"].as_bool().unwrap_or(false);
    let fee = tx["fee"].as_i64().unwrap_or(0);

    let mut value_in: i64 = 0;
    let mut value_out: i64 = 0;
    if let Some(vins) = tx["vin"].as_array() {
        for vin in vins {
            if let Some(prevout) = vin.get("prevout") {
                if prevout["scriptpubkey_address"].as_str() == Some(address) {
                    value_in += prevout["value"].as_i64().unwrap_or(0);
                }
            }
        }
    }
    if let Some(vouts) = tx["vout"].as_array() {
        for vout in vouts {
            if vout["scriptpubkey_address"].as_str() == Some(address) {
                value_out += vout["value"].as_i64().unwrap_or(0);
            }
        }
    }
    let (direction, amount) = if value_out >= value_in {
        ("in".to_string(), value_out - value_in)
    } else {
        ("out".to_string(), value_in - value_out)
    };

    Some(MaestroAddressTx {
        txid,
        direction,
        amount,
        fee,
        block_height,
        block_time,
        confirmed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_address_tx_in_direction() {
        let raw = json!({
            "txid": "abc",
            "status": {"confirmed": true, "block_height": 900000, "block_time": 1700000000},
            "fee": 250,
            "vin": [{"prevout": {"scriptpubkey_address": "other", "value": 500}}],
            "vout": [{"scriptpubkey_address": "me", "value": 800}]
        });
        let parsed = parse_address_tx(&raw, "me").unwrap();
        assert_eq!(parsed.txid, "abc");
        assert_eq!(parsed.direction, "in");
        assert_eq!(parsed.amount, 800);
        assert_eq!(parsed.block_height, Some(900000));
        assert!(parsed.confirmed);
    }

    #[test]
    fn parse_address_tx_out_direction() {
        let raw = json!({
            "txid": "def",
            "status": {"confirmed": false},
            "fee": 100,
            "vin": [{"prevout": {"scriptpubkey_address": "me", "value": 1000}}],
            "vout": [
                {"scriptpubkey_address": "recipient", "value": 700},
                {"scriptpubkey_address": "me", "value": 200}
            ]
        });
        let parsed = parse_address_tx(&raw, "me").unwrap();
        assert_eq!(parsed.direction, "out");
        assert_eq!(parsed.amount, 800); // 1000 in − 200 change = 800 out
        assert_eq!(parsed.block_height, None);
        assert!(!parsed.confirmed);
    }
}
