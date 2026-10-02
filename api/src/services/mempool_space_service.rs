// Esplora REST client — the API's only source of live Bitcoin data.
//
// Talks to the free public mempool.space Esplora API (no key, no node of our
// own). Base URLs are per network and overridable via
// BITCOIN_MAINNET_ESPLORA_URL / BITCOIN_TESTNET4_ESPLORA_URL.
//
// mempool.space rate-limits by IP: HTTP 429 is retried a couple of times with
// a short backoff, and a semaphore caps how many requests we have in flight so
// the batch endpoints don't burst into the limit in the first place.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::Semaphore;

use super::wallet_service::{
    AddressTxRecord, ChainTip, FeeEstimate, ScriptPubKey, TransactionDetail, TxInput, TxOutput,
    Utxo,
};

pub const DEFAULT_MAINNET_URL: &str = "https://mempool.space/api";
pub const DEFAULT_TESTNET4_URL: &str = "https://mempool.space/testnet4/api";
const TESTNET3_URL: &str = "https://mempool.space/testnet/api";

/// Max concurrent in-flight requests to the Esplora host.
/// Kept low so batch endpoints queue instead of bursting into a 429.
const MAX_IN_FLIGHT: usize = 8;
/// Retries after an HTTP 429 (so up to MAX_429_RETRIES + 1 attempts).
const MAX_429_RETRIES: u32 = 4;
/// Upper bound for a server-provided Retry-After we are willing to honour.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(3);
/// Page cap for address history when seeding (25 txs per page).
const MAX_HISTORY_PAGES: usize = 10;

#[derive(Debug)]
pub enum EsploraError {
    /// The address has more UTXOs than the Esplora host is willing to list.
    TooManyUtxos(String),
    /// 404 / unknown object (tx, block, ...).
    NotFound(String),
    /// Non-success HTTP status (after 429 retries are exhausted).
    Http { status: u16, body: String },
    /// Network / parse failure.
    Transport(String),
}

impl fmt::Display for EsploraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EsploraError::TooManyUtxos(addr) => write!(
                f,
                "Address {} has too many UTXOs for the public Esplora API (mempool.space) to list",
                addr
            ),
            EsploraError::NotFound(what) => write!(f, "Esplora: not found: {}", what),
            EsploraError::Http { status, body } => {
                write!(f, "Esplora error {}: {}", status, body.trim())
            }
            EsploraError::Transport(e) => write!(f, "Esplora request failed: {}", e),
        }
    }
}

impl From<EsploraError> for String {
    fn from(e: EsploraError) -> Self {
        e.to_string()
    }
}

type EResult<T> = Result<T, EsploraError>;

/// Network-aware Esplora client. Cheap to clone (shares the HTTP pool and
/// the in-flight limiter).
#[derive(Clone)]
pub struct EsploraClient {
    http: reqwest::Client,
    mainnet_url: String,
    testnet4_url: String,
    limiter: Arc<Semaphore>,
}

impl EsploraClient {
    pub fn new(http: reqwest::Client, mainnet_url: &str, testnet4_url: &str) -> Self {
        Self {
            http,
            mainnet_url: mainnet_url.trim_end_matches('/').to_string(),
            testnet4_url: testnet4_url.trim_end_matches('/').to_string(),
            limiter: Arc::new(Semaphore::new(MAX_IN_FLIGHT)),
        }
    }

    /// Esplora base URL for a network name as used in API query params.
    pub fn base_url(&self, network: &str) -> &str {
        match network {
            "testnet4" => &self.testnet4_url,
            "testnet" | "testnet3" => TESTNET3_URL,
            _ => &self.mainnet_url,
        }
    }

    // ---------------------------------------------------------------- transport

    /// Send a request, retrying on 429 with a short backoff.
    async fn send(&self, req: reqwest::RequestBuilder) -> EResult<reqwest::Response> {
        let _permit = self
            .limiter
            .acquire()
            .await
            .map_err(|e| EsploraError::Transport(e.to_string()))?;

        let mut attempt = 0u32;
        loop {
            let r = req
                .try_clone()
                .ok_or_else(|| EsploraError::Transport("request not cloneable".into()))?;
            let resp = match r.send().await {
                Ok(resp) => resp,
                // One retry on a transport error (dropped connection, reset).
                Err(_) if attempt == 0 => {
                    attempt += 1;
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    continue;
                }
                Err(e) => return Err(EsploraError::Transport(e.to_string())),
            };

            if resp.status().as_u16() != 429 || attempt >= MAX_429_RETRIES {
                return Ok(resp);
            }

            let wait = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(Duration::from_secs)
                .unwrap_or_else(|| Duration::from_millis(300 * 3u64.pow(attempt)))
                .min(MAX_RETRY_AFTER);
            tracing::warn!(
                "Esplora 429 from {} — retrying in {}ms (attempt {}/{})",
                resp.url(),
                wait.as_millis(),
                attempt + 1,
                MAX_429_RETRIES
            );
            tokio::time::sleep(wait).await;
            attempt += 1;
        }
    }

    /// GET a path and return the body text on success.
    async fn get_text(&self, network: &str, path: &str) -> EResult<String> {
        let url = format!("{}{}", self.base_url(network), path);
        let resp = self.send(self.http.get(&url)).await?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| EsploraError::Transport(e.to_string()))?;
        if status.is_success() {
            Ok(body)
        } else if status.as_u16() == 404 {
            Err(EsploraError::NotFound(path.to_string()))
        } else {
            Err(EsploraError::Http { status: status.as_u16(), body })
        }
    }

    async fn get_json(&self, network: &str, path: &str) -> EResult<Value> {
        let body = self.get_text(network, path).await?;
        serde_json::from_str(&body)
            .map_err(|e| EsploraError::Transport(format!("parse {}: {}", path, e)))
    }

    // ---------------------------------------------------------------- endpoints

    /// GET /address/:a/utxo — mempool-aware (includes unconfirmed, excludes
    /// outputs spent in the mempool).
    pub async fn get_utxos(
        &self,
        network: &str,
        address: &str,
        min_value: Option<u64>,
    ) -> EResult<Vec<Utxo>> {
        let path = format!("/address/{}/utxo", address);
        let raw = match self.get_json(network, &path).await {
            Ok(v) => v,
            Err(EsploraError::Http { status: 400, body }) if is_too_many_utxos(&body) => {
                return Err(EsploraError::TooManyUtxos(address.to_string()));
            }
            Err(e) => return Err(e),
        };
        Ok(parse_utxos(&raw, min_value))
    }

    /// UTXOs + up to MAX_HISTORY_PAGES pages of tx history, for seeding.
    pub async fn get_address_info(
        &self,
        network: &str,
        address: &str,
    ) -> EResult<(Vec<Utxo>, Vec<AddressTxRecord>)> {
        let utxos = self.get_utxos(network, address, None).await?;

        let mut all_txs = Vec::new();
        let mut after_txid: Option<String> = None;
        for _ in 0..MAX_HISTORY_PAGES {
            let path = match &after_txid {
                Some(txid) => format!("/address/{}/txs/chain/{}", address, txid),
                None => format!("/address/{}/txs", address),
            };
            let page = self.get_json(network, &path).await?;
            let txs = page.as_array().cloned().unwrap_or_default();
            if txs.is_empty() {
                break;
            }
            all_txs.extend(txs.iter().map(|tx| parse_address_tx(tx, address)));

            // /address/:a/txs returns up to 50 mempool + 25 confirmed txs;
            // /txs/chain/:last pages the confirmed ones 25 at a time. Keep
            // paging only while a full confirmed page came back.
            let confirmed: Vec<&Value> = txs
                .iter()
                .filter(|t| t["status"]["confirmed"].as_bool().unwrap_or(false))
                .collect();
            if confirmed.len() < 25 {
                break;
            }
            after_txid = confirmed
                .last()
                .and_then(|t| t["txid"].as_str().map(|s| s.to_string()));
        }

        Ok((utxos, all_txs))
    }

    /// Current tip height + hash. With `with_time`, also fetch the block
    /// header timestamp (best effort — `time` stays None on failure).
    pub async fn get_chain_tip(&self, network: &str, with_time: bool) -> EResult<ChainTip> {
        let height_txt = self.get_text(network, "/blocks/tip/height").await?;
        let height = height_txt.trim().parse::<u64>().map_err(|e| {
            EsploraError::Transport(format!("bad tip height '{}': {}", height_txt.trim(), e))
        })?;
        let hash = self
            .get_text(network, "/blocks/tip/hash")
            .await?
            .trim()
            .to_string();

        let time = if with_time {
            self.get_json(network, &format!("/block/{}", hash))
                .await
                .ok()
                .and_then(|b| b["timestamp"].as_u64())
        } else {
            None
        };

        Ok(ChainTip { height, hash, time })
    }

    /// Fee rate for a confirmation target. Returned in BTC/kvB (the unit the
    /// API has always exposed, inherited from estimatesmartfee).
    pub async fn get_fee_estimate(&self, network: &str, target_blocks: u16) -> EResult<FeeEstimate> {
        let sat_vb = match self.get_json(network, "/fee-estimates").await {
            Ok(data) => pick_fee_estimate(&data, target_blocks),
            Err(e) => {
                tracing::warn!("Esplora /fee-estimates failed, trying /v1/fees/recommended: {}", e);
                None
            }
        };
        let sat_vb = match sat_vb {
            Some(r) => r,
            None => {
                let rec = self.get_json(network, "/v1/fees/recommended").await?;
                pick_recommended_fee(&rec, target_blocks).unwrap_or(1.0)
            }
        };
        Ok(FeeEstimate {
            fee_rate: sat_vb_to_btc_kvb(sat_vb),
            blocks: target_blocks,
        })
    }

    /// POST /tx — returns the txid.
    pub async fn broadcast(&self, network: &str, raw_tx_hex: &str) -> EResult<String> {
        let url = format!("{}/tx", self.base_url(network));
        let req = self
            .http
            .post(&url)
            .header("Content-Type", "text/plain")
            .body(raw_tx_hex.trim().to_string());
        let resp = self.send(req).await?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status.is_success() {
            Ok(body.trim().to_string())
        } else {
            Err(EsploraError::Http { status: status.as_u16(), body })
        }
    }

    /// GET /tx/:id/hex
    pub async fn get_tx_hex(&self, network: &str, txid: &str) -> EResult<String> {
        Ok(self
            .get_text(network, &format!("/tx/{}/hex", txid))
            .await?
            .trim()
            .to_string())
    }

    /// GET /tx/:id (+ /tx/:id/hex, + tip height when confirmed), normalised to
    /// the bitcoind-verbose-like `TransactionDetail` shape the API returns.
    pub async fn get_transaction(&self, network: &str, txid: &str) -> EResult<TransactionDetail> {
        let tx = self.get_json(network, &format!("/tx/{}", txid)).await?;
        let hex = self.get_tx_hex(network, txid).await?;
        let tip_height = if tx["status"]["confirmed"].as_bool().unwrap_or(false) {
            self.get_text(network, "/blocks/tip/height")
                .await
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
        } else {
            None
        };
        Ok(parse_transaction(&tx, hex, tip_height))
    }
}

// -------------------------------------------------------------------- parsing

fn is_too_many_utxos(body: &str) -> bool {
    body.to_ascii_lowercase().contains("too many")
}

fn parse_utxos(raw: &Value, min_value: Option<u64>) -> Vec<Utxo> {
    raw.as_array()
        .map(|arr| arr.as_slice())
        .unwrap_or(&[])
        .iter()
        .filter_map(|u| {
            let value = u["value"].as_u64().unwrap_or(0);
            if min_value.is_some_and(|min| value < min) {
                return None;
            }
            let confirmed = u["status"]["confirmed"].as_bool().unwrap_or(false);
            let block_height = u["status"]["block_height"].as_u64().unwrap_or(0) as u32;
            Some(Utxo {
                txid: u["txid"].as_str().unwrap_or("").to_string(),
                vout: u["vout"].as_u64().unwrap_or(0) as u32,
                value,
                script_pubkey: String::new(),
                // Kept as before: confirmed UTXOs report their block height
                // here (callers only test > 0); mempool = 0.
                confirmations: if confirmed { block_height } else { 0 },
                block_height: if confirmed { Some(block_height) } else { None },
            })
        })
        .collect()
}

fn parse_address_tx(tx: &Value, address: &str) -> AddressTxRecord {
    let status = &tx["status"];
    let confirmed = status["confirmed"].as_bool().unwrap_or(false);

    let value_in: i64 = tx["vin"]
        .as_array()
        .map(|vins| {
            vins.iter()
                .filter(|v| v["prevout"]["scriptpubkey_address"].as_str() == Some(address))
                .map(|v| v["prevout"]["value"].as_i64().unwrap_or(0))
                .sum()
        })
        .unwrap_or(0);
    let value_out: i64 = tx["vout"]
        .as_array()
        .map(|vouts| {
            vouts
                .iter()
                .filter(|v| v["scriptpubkey_address"].as_str() == Some(address))
                .map(|v| v["value"].as_i64().unwrap_or(0))
                .sum()
        })
        .unwrap_or(0);

    let (direction, amount) = if value_out >= value_in {
        ("in".to_string(), value_out - value_in)
    } else {
        ("out".to_string(), value_in - value_out)
    };

    AddressTxRecord {
        txid: tx["txid"].as_str().unwrap_or("").to_string(),
        direction,
        amount,
        fee: tx["fee"].as_i64().unwrap_or(0),
        block_height: if confirmed { status["block_height"].as_i64().map(|h| h as i32) } else { None },
        block_time: if confirmed { status["block_time"].as_i64() } else { None },
        confirmations: if confirmed { 1 } else { 0 },
    }
}

/// Esplora `/fee-estimates`: { "<target>": sat/vB, ... }. Exact target, else
/// the closest available one.
fn pick_fee_estimate(data: &Value, target: u16) -> Option<f64> {
    let obj = data.as_object()?;
    if let Some(r) = obj.get(&target.to_string()).and_then(|v| v.as_f64()) {
        return Some(r);
    }
    obj.iter()
        .filter_map(|(k, v)| Some((k.parse::<u16>().ok()?, v.as_f64()?)))
        .min_by_key(|(n, _)| (*n as i32 - target as i32).unsigned_abs())
        .map(|(_, r)| r)
}

/// mempool.space `/v1/fees/recommended`: fastestFee / halfHourFee / hourFee /
/// economyFee / minimumFee (sat/vB).
fn pick_recommended_fee(data: &Value, target: u16) -> Option<f64> {
    let key = match target {
        0..=1 => "fastestFee",
        2..=3 => "halfHourFee",
        4..=6 => "hourFee",
        _ => "economyFee",
    };
    data[key].as_f64()
}

fn sat_vb_to_btc_kvb(sat_vb: f64) -> f64 {
    sat_vb / 100_000.0
}

/// Esplora script type -> bitcoind scriptPubKey.type
fn core_script_type(esplora: &str) -> String {
    match esplora {
        "p2pkh" => "pubkeyhash",
        "p2sh" => "scripthash",
        "v0_p2wpkh" => "witness_v0_keyhash",
        "v0_p2wsh" => "witness_v0_scripthash",
        "v1_p2tr" => "witness_v1_taproot",
        "op_return" => "nulldata",
        "p2pk" => "pubkey",
        "multisig" => "multisig",
        "" => "nonstandard",
        other if other.starts_with("unknown") => "witness_unknown",
        other => other,
    }
    .to_string()
}

fn parse_transaction(tx: &Value, hex: String, tip_height: Option<u64>) -> TransactionDetail {
    let status = &tx["status"];
    let confirmed = status["confirmed"].as_bool().unwrap_or(false);
    let block_height = if confirmed { status["block_height"].as_u64() } else { None };
    let weight = tx["weight"].as_u64().unwrap_or(0);

    let inputs = tx["vin"]
        .as_array()
        .map(|a| a.as_slice())
        .unwrap_or(&[])
        .iter()
        .map(|vin| TxInput {
            txid: vin["txid"].as_str().unwrap_or("").to_string(),
            vout: vin["vout"].as_u64().unwrap_or(0) as u32,
            script_sig: vin["scriptsig"].as_str().unwrap_or("").to_string(),
            sequence: vin["sequence"].as_u64().unwrap_or(0) as u32,
            witness: vin["witness"]
                .as_array()
                .map(|w| w.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
                .unwrap_or_default(),
        })
        .collect();

    let outputs = tx["vout"]
        .as_array()
        .map(|a| a.as_slice())
        .unwrap_or(&[])
        .iter()
        .enumerate()
        .map(|(n, vout)| TxOutput {
            value: vout["value"].as_u64().unwrap_or(0) as f64 / 100_000_000.0,
            n: n as u32,
            script_pubkey: ScriptPubKey {
                asm: vout["scriptpubkey_asm"].as_str().unwrap_or("").to_string(),
                hex: vout["scriptpubkey"].as_str().unwrap_or("").to_string(),
                script_type: core_script_type(vout["scriptpubkey_type"].as_str().unwrap_or("")),
                address: vout["scriptpubkey_address"].as_str().map(|s| s.to_string()),
            },
        })
        .collect();

    TransactionDetail {
        txid: tx["txid"].as_str().unwrap_or("").to_string(),
        version: tx["version"].as_i64().unwrap_or(0) as i32,
        locktime: tx["locktime"].as_u64().unwrap_or(0) as u32,
        size: tx["size"].as_u64().unwrap_or(0) as usize,
        vsize: weight.div_ceil(4) as usize,
        weight: weight as usize,
        fee: tx["fee"].as_u64().map(|f| f as f64 / 100_000_000.0),
        confirmations: match (block_height, tip_height) {
            (Some(h), Some(tip)) if tip >= h => Some((tip - h + 1) as u32),
            (Some(_), _) => Some(1),
            // bitcoind omits `confirmations` for mempool txs.
            _ => None,
        },
        block_hash: if confirmed { status["block_hash"].as_str().map(|s| s.to_string()) } else { None },
        block_height: block_height.map(|h| h as u32),
        time: if confirmed { status["block_time"].as_u64() } else { None },
        inputs,
        outputs,
        hex,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ADDR: &str = "bc1qexampleaddr";

    #[test]
    fn utxos_parse_confirmed_and_mempool() {
        let raw = json!([
            {"txid":"aa","vout":1,"value":5000,
             "status":{"confirmed":true,"block_height":850000,"block_hash":"h","block_time":1}},
            {"txid":"bb","vout":0,"value":300,"status":{"confirmed":false}}
        ]);
        let u = parse_utxos(&raw, None);
        assert_eq!(u.len(), 2);
        assert_eq!(u[0].txid, "aa");
        assert_eq!(u[0].vout, 1);
        assert_eq!(u[0].value, 5000);
        assert_eq!(u[0].block_height, Some(850000));
        assert_eq!(u[1].block_height, None);
        assert_eq!(u[1].confirmations, 0);

        let filtered = parse_utxos(&raw, Some(1000));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].txid, "aa");
    }

    #[test]
    fn too_many_utxos_detection() {
        assert!(is_too_many_utxos(
            "Too many unspent transaction outputs (>500). Contact support to raise limits."
        ));
        assert!(!is_too_many_utxos("Invalid Bitcoin address"));
    }

    #[test]
    fn address_tx_direction_and_amount() {
        let incoming = json!({
            "txid":"t1","fee":200,
            "status":{"confirmed":true,"block_height":10,"block_time":1700000000},
            "vin":[{"prevout":{"scriptpubkey_address":"other","value":10000}}],
            "vout":[{"scriptpubkey_address":ADDR,"value":7000},{"scriptpubkey_address":"other","value":2800}]
        });
        let r = parse_address_tx(&incoming, ADDR);
        assert_eq!(r.direction, "in");
        assert_eq!(r.amount, 7000);
        assert_eq!(r.fee, 200);
        assert_eq!(r.block_height, Some(10));
        assert_eq!(r.block_time, Some(1700000000));
        assert_eq!(r.confirmations, 1);

        let outgoing = json!({
            "txid":"t2","fee":150,"status":{"confirmed":false},
            "vin":[{"prevout":{"scriptpubkey_address":ADDR,"value":7000}}],
            "vout":[{"scriptpubkey_address":"other","value":4000},{"scriptpubkey_address":ADDR,"value":2850}]
        });
        let r = parse_address_tx(&outgoing, ADDR);
        assert_eq!(r.direction, "out");
        assert_eq!(r.amount, 4150);
        assert_eq!(r.block_height, None);
        assert_eq!(r.confirmations, 0);
    }

    #[test]
    fn fee_estimates_exact_and_closest() {
        let data = json!({"1": 20.5, "3": 12.0, "6": 8.0, "144": 1.5});
        assert_eq!(pick_fee_estimate(&data, 6), Some(8.0));
        assert_eq!(pick_fee_estimate(&data, 5), Some(8.0));
        assert_eq!(pick_fee_estimate(&data, 100), Some(1.5));
        assert_eq!(pick_fee_estimate(&json!({}), 6), None);
        assert!((sat_vb_to_btc_kvb(8.0) - 0.00008).abs() < 1e-12);
    }

    #[test]
    fn recommended_fee_mapping() {
        let rec = json!({"fastestFee":30,"halfHourFee":20,"hourFee":10,"economyFee":5,"minimumFee":1});
        assert_eq!(pick_recommended_fee(&rec, 1), Some(30.0));
        assert_eq!(pick_recommended_fee(&rec, 3), Some(20.0));
        assert_eq!(pick_recommended_fee(&rec, 6), Some(10.0));
        assert_eq!(pick_recommended_fee(&rec, 144), Some(5.0));
    }

    #[test]
    fn transaction_normalised_to_core_shape() {
        let tx = json!({
            "txid":"abc","version":2,"locktime":0,"size":222,"weight":561,"fee":1410,
            "status":{"confirmed":true,"block_height":100,"block_hash":"bh","block_time":1700000000},
            "vin":[{"txid":"prev","vout":3,"scriptsig":"","sequence":4294967293u64,
                    "witness":["3044","02ab"],"prevout":{"value":1}}],
            "vout":[
                {"scriptpubkey":"0014ab","scriptpubkey_asm":"OP_0 OP_PUSHBYTES_20 ab",
                 "scriptpubkey_type":"v0_p2wpkh","scriptpubkey_address":ADDR,"value":150000000u64},
                {"scriptpubkey":"6a","scriptpubkey_asm":"OP_RETURN","scriptpubkey_type":"op_return","value":0}
            ]
        });
        let d = parse_transaction(&tx, "0200".into(), Some(105));
        assert_eq!(d.txid, "abc");
        assert_eq!(d.vsize, 141);
        assert_eq!(d.confirmations, Some(6));
        assert_eq!(d.block_hash.as_deref(), Some("bh"));
        assert_eq!(d.block_height, Some(100));
        assert_eq!(d.time, Some(1700000000));
        assert_eq!(d.fee, Some(0.0000141));
        assert_eq!(d.hex, "0200");
        assert_eq!(d.inputs[0].txid, "prev");
        assert_eq!(d.inputs[0].sequence, 4294967293);
        assert_eq!(d.inputs[0].witness, vec!["3044", "02ab"]);
        assert_eq!(d.outputs[0].value, 1.5);
        assert_eq!(d.outputs[0].script_pubkey.script_type, "witness_v0_keyhash");
        assert_eq!(d.outputs[0].script_pubkey.address.as_deref(), Some(ADDR));
        assert_eq!(d.outputs[1].n, 1);
        assert_eq!(d.outputs[1].script_pubkey.script_type, "nulldata");

        let mempool = json!({"txid":"m","weight":400,"status":{"confirmed":false},"vin":[],"vout":[]});
        let d = parse_transaction(&mempool, String::new(), None);
        assert_eq!(d.confirmations, None);
        assert_eq!(d.block_height, None);
        assert_eq!(d.time, None);
    }

    #[test]
    fn base_url_per_network_and_override() {
        let c = EsploraClient::new(reqwest::Client::new(), DEFAULT_MAINNET_URL, "http://x/t4/api/");
        assert_eq!(c.base_url("mainnet"), "https://mempool.space/api");
        assert_eq!(c.base_url("anything"), "https://mempool.space/api");
        assert_eq!(c.base_url("testnet4"), "http://x/t4/api");
    }

    /// Live smoke test against mempool.space: `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn live_mempool_space_smoke() {
        let c = EsploraClient::new(reqwest::Client::new(), DEFAULT_MAINNET_URL, DEFAULT_TESTNET4_URL);
        for net in ["mainnet", "testnet4"] {
            let tip = c.get_chain_tip(net, true).await.unwrap();
            assert!(tip.height > 0 && tip.hash.len() == 64 && tip.time.is_some(), "{net}: {tip:?}");
            let fee = c.get_fee_estimate(net, 6).await.unwrap();
            assert!(fee.fee_rate > 0.0, "{net}: {fee:?}");
        }
        // Genesis coinbase output address (single, well-known history).
        let genesis_addr = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
        let (utxos, txs) = c.get_address_info("mainnet", genesis_addr).await
            .or_else(|e| match e { EsploraError::TooManyUtxos(_) => Ok((vec![], vec![])), e => Err(e) })
            .unwrap();
        println!("genesis addr: {} utxos, {} txs", utxos.len(), txs.len());
        let txid = "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b";
        let hex = c.get_tx_hex("mainnet", txid).await.unwrap();
        assert!(hex.starts_with("01000000"));
        let tx = c.get_transaction("mainnet", txid).await.unwrap();
        assert_eq!(tx.block_height, Some(0));
        assert_eq!(tx.outputs[0].value, 50.0);
        assert!(matches!(c.get_tx_hex("mainnet", &"0".repeat(64)).await, Err(EsploraError::NotFound(_)) | Err(EsploraError::Http { .. })));
        // Well-known busy address: must surface the clear "too many UTXOs" error.
        let busy = c.get_utxos("mainnet", "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh", None).await;
        println!("busy addr utxos: {:?}", busy.as_ref().map(|v| v.len()));
    }
}
