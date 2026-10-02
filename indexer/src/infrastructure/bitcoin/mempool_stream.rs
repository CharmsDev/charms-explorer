//! Live mempool feed from the mempool.space websocket (`track-mempool`).
//!
//! The gateway pushes every tx entering the mempool as full Esplora JSON
//! (inputs with witness, outputs with scripts), so we rebuild the raw tx
//! locally instead of fetching each one — the only way to follow the
//! mainnet mempool without a node or a paid API.

use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bitcoincore_rpc::bitcoin::{
    absolute::LockTime, consensus::serialize, OutPoint, ScriptBuf, Sequence, Transaction, TxIn,
    TxOut, Txid, Witness,
};
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

use crate::utils::logging;

/// Queue cap: if the consumer stalls we drop the oldest entries rather than
/// grow without bound (blocks still pick those txs up on confirmation).
const MAX_QUEUE: usize = 50_000;

/// A tx announced by the feed. `hex` is None when the JSON could not be
/// rebuilt into a tx with the announced txid — the consumer fetches it.
#[derive(Debug, Clone)]
pub struct StreamedTx {
    pub txid: String,
    pub hex: Option<String>,
}

#[derive(Debug)]
pub struct MempoolStream {
    queue: Mutex<VecDeque<StreamedTx>>,
    connected: AtomicBool,
}

impl MempoolStream {
    /// Start the feed in the background; reconnects forever.
    pub fn spawn(network: String, ws_url: String) -> Arc<Self> {
        let stream = Arc::new(Self {
            queue: Mutex::new(VecDeque::new()),
            connected: AtomicBool::new(false),
        });
        let s = stream.clone();
        tokio::spawn(async move { s.run(network, ws_url).await });
        stream
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub async fn drain(&self, max: usize) -> Vec<StreamedTx> {
        let mut q = self.queue.lock().await;
        let n = max.min(q.len());
        q.drain(..n).collect()
    }

    async fn run(self: Arc<Self>, network: String, ws_url: String) {
        let mut backoff = 1u64;
        loop {
            if let Err(e) = self.session(&network, &ws_url).await {
                logging::log_warning(&format!("[{}] mempool ws error: {}", network, e));
            }
            // A session that got connected earns a fast retry.
            if self.connected.swap(false, Ordering::Relaxed) {
                backoff = 1;
            }
            tokio::time::sleep(Duration::from_secs(backoff)).await;
            backoff = (backoff * 2).min(60);
        }
    }

    async fn session(&self, network: &str, ws_url: &str) -> Result<(), String> {
        let (ws, _) = tokio_tungstenite::connect_async(ws_url)
            .await
            .map_err(|e| e.to_string())?;
        let (mut tx, mut rx) = ws.split();
        tx.send(Message::Text(r#"{"track-mempool":true}"#.into()))
            .await
            .map_err(|e| e.to_string())?;
        self.connected.store(true, Ordering::Relaxed);
        logging::log_info(&format!("[{}] mempool ws connected: {}", network, ws_url));

        // Quiet networks (testnet4) can go minutes without a push; ping on
        // silence and only give up when even pongs stop coming back.
        let idle = Duration::from_secs(60);
        let mut silent = 0u32;
        loop {
            let msg = match tokio::time::timeout(idle, rx.next()).await {
                Err(_) => {
                    silent += 1;
                    if silent >= 3 {
                        return Err("no traffic, reconnecting".into());
                    }
                    tx.send(Message::Ping(Vec::new().into()))
                        .await
                        .map_err(|e| e.to_string())?;
                    continue;
                }
                Ok(None) => return Err("closed".into()),
                Ok(Some(Err(e))) => return Err(e.to_string()),
                Ok(Some(Ok(m))) => m,
            };
            silent = 0;
            let text = match msg {
                Message::Text(t) => t,
                Message::Ping(p) => {
                    let _ = tx.send(Message::Pong(p)).await;
                    continue;
                }
                Message::Close(_) => return Err("closed by server".into()),
                _ => continue,
            };
            let Ok(v) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            let Some(added) = v["mempool-transactions"]["added"].as_array() else {
                continue;
            };
            let txs: Vec<StreamedTx> = added.iter().filter_map(streamed_tx).collect();
            if txs.is_empty() {
                continue;
            }
            let mut q = self.queue.lock().await;
            q.extend(txs);
            while q.len() > MAX_QUEUE {
                q.pop_front();
            }
        }
    }
}

fn streamed_tx(v: &Value) -> Option<StreamedTx> {
    let txid = v["txid"].as_str()?.to_string();
    let hex = rebuild_tx(v)
        .filter(|tx| tx.txid().to_string() == txid)
        .map(|tx| hex::encode(serialize(&tx)));
    Some(StreamedTx { txid, hex })
}

/// Rebuild a consensus tx from Esplora JSON.
fn rebuild_tx(v: &Value) -> Option<Transaction> {
    let mut input = Vec::new();
    for vin in v["vin"].as_array()? {
        let witness: Vec<Vec<u8>> = match vin["witness"].as_array() {
            Some(items) => items
                .iter()
                .map(|w| hex::decode(w.as_str().unwrap_or("")).ok())
                .collect::<Option<_>>()?,
            None => Vec::new(),
        };
        input.push(TxIn {
            previous_output: OutPoint::new(
                Txid::from_str(vin["txid"].as_str()?).ok()?,
                vin["vout"].as_u64()? as u32,
            ),
            script_sig: ScriptBuf::from_bytes(hex::decode(vin["scriptsig"].as_str()?).ok()?),
            sequence: Sequence(vin["sequence"].as_u64()? as u32),
            witness: Witness::from_slice(&witness),
        });
    }
    let mut output = Vec::new();
    for vout in v["vout"].as_array()? {
        output.push(TxOut {
            value: vout["value"].as_u64()?,
            script_pubkey: ScriptBuf::from_bytes(hex::decode(vout["scriptpubkey"].as_str()?).ok()?),
        });
    }
    Some(Transaction {
        version: v["version"].as_i64()? as i32,
        lock_time: LockTime::from_consensus(v["locktime"].as_u64()? as u32),
        input,
        output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rebuilds_segwit_tx_with_matching_txid() {
        // mempool.space JSON for a real 1-in 2-out p2wpkh tx.
        let v = json!({
            "txid": "a75dae984aafc08400ebe8e2fccc18ddd349f7b99078361cf86ca6ca60a42a09",
            "version": 2, "locktime": 0,
            "vin": [{
                "txid": "e972da182d9767c7b70d3ac3ec7d0360000a24682c0a7cb89ecfa500ecb82f07",
                "vout": 0, "scriptsig": "", "sequence": 4294967295u64,
                "witness": [
                    "30440220315c6eccbdf39790c5d2d183717d8f81e18ed926f2a047840992c09acf4a007e022011d953f8446b6469657aab331141b1e70399e4d38930a2af1296a3a2535dd77f01",
                    "0299dff2323d8eec673df5d59c4c797f12972328277e68c04a11d7c034592e4e5e"
                ]
            }],
            "vout": [
                {"scriptpubkey": "0014ef28fc06f4010edfae4110d4d0e9be6e819da286", "value": 68455},
                {"scriptpubkey": "6a5d1214011400ff7f818cec82d08bc0a88281d215", "value": 0}
            ]
        });
        let s = streamed_tx(&v).unwrap();
        assert!(s.hex.is_some(), "txid mismatch after rebuild");
    }

    #[test]
    fn bad_json_falls_back_to_fetch() {
        let v = json!({"txid": "00", "vin": "nope"});
        let s = streamed_tx(&v).unwrap();
        assert!(s.hex.is_none());
    }
}
