// Wallet service for Bitcoin RPC operations
// Phase 1: Direct Bitcoin node access for wallet extension

use std::sync::Arc;

use bitcoincore_rpc::{Client, RpcApi};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- Response types ---

#[derive(Debug, Serialize, Deserialize)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub value: u64,
    pub script_pubkey: String,
    pub confirmations: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddressBalance {
    pub address: String,
    pub confirmed: u64,
    pub unconfirmed: i64,
    pub utxo_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionDetail {
    pub txid: String,
    pub version: i32,
    pub locktime: u32,
    pub size: usize,
    pub vsize: usize,
    pub weight: usize,
    pub fee: Option<f64>,
    pub confirmations: Option<u32>,
    pub block_hash: Option<String>,
    pub block_height: Option<u32>,
    pub time: Option<u64>,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub hex: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxInput {
    pub txid: String,
    pub vout: u32,
    pub script_sig: String,
    pub sequence: u32,
    pub witness: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxOutput {
    pub value: f64,
    pub n: u32,
    pub script_pubkey: ScriptPubKey,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptPubKey {
    pub asm: String,
    pub hex: String,
    #[serde(rename = "type")]
    pub script_type: String,
    pub address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BroadcastResult {
    pub txid: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeeEstimate {
    pub fee_rate: f64,
    pub blocks: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainTip {
    pub height: u64,
    pub hash: String,
    pub time: Option<u64>,
}

// --- Service ---

pub struct WalletService;

impl WalletService {
    // ==================== Database methods (address_utxos index) ====================

    /// Get UTXOs from our own address_utxos table (populated by indexer)
    pub async fn get_utxos_db(
        utxo_repo: &crate::db::repositories::UtxoRepository,
        address: &str,
        network: &str,
        chain_height: Option<u64>,
    ) -> Result<Vec<Utxo>, String> {
        let rows = utxo_repo.get_by_address(address, network).await?;
        let tip = chain_height.unwrap_or(0);

        let utxos = rows
            .into_iter()
            .map(|r| {
                let confirmations = if tip > 0 && r.block_height > 0 {
                    (tip as i64 - r.block_height as i64 + 1).max(0) as u32
                } else {
                    0
                };
                Utxo {
                    txid: r.txid,
                    vout: r.vout as u32,
                    value: r.value as u64,
                    script_pubkey: r.script_pubkey,
                    confirmations,
                }
            })
            .collect();

        Ok(utxos)
    }

    /// Get balance from our own address_utxos table
    pub async fn get_balance_db(
        utxo_repo: &crate::db::repositories::UtxoRepository,
        address: &str,
        network: &str,
    ) -> Result<AddressBalance, String> {
        let rows = utxo_repo.get_by_address(address, network).await?;
        let confirmed: u64 = rows.iter().map(|r| r.value as u64).sum();
        let utxo_count = rows.len();

        Ok(AddressBalance {
            address: address.to_string(),
            confirmed,
            unconfirmed: 0,
            utxo_count,
        })
    }

    // ==================== QuickNode methods (mainnet) ====================

    /// Get UTXOs via QuickNode bb_getutxos (indexed, instant response)
    pub async fn get_utxos_quicknode(
        http_client: &reqwest::Client,
        quicknode_url: &str,
        address: &str,
    ) -> Result<Vec<Utxo>, String> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "bb_getutxos",
            "params": [address, {"confirmed": true}],
            "id": 1
        });

        let resp = http_client
            .post(quicknode_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("QuickNode request failed: {}", e))?;

        let data: Value = resp
            .json()
            .await
            .map_err(|e| format!("QuickNode response parse failed: {}", e))?;

        if let Some(err) = data.get("error").filter(|e| !e.is_null()) {
            return Err(format!("QuickNode error: {}", err));
        }

        let utxos = data["result"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|u| {
                let value_str = u["value"].as_str().unwrap_or("0");
                let value: u64 = value_str.parse().unwrap_or(0);
                Utxo {
                    txid: u["txid"].as_str().unwrap_or("").to_string(),
                    vout: u["vout"].as_u64().unwrap_or(0) as u32,
                    value,
                    script_pubkey: String::new(),
                    confirmations: u["confirmations"].as_u64().unwrap_or(0) as u32,
                }
            })
            .collect();

        Ok(utxos)
    }

    /// Get chain tip via QuickNode (getblockcount + getbestblockhash)
    pub async fn get_chain_tip_quicknode(
        http_client: &reqwest::Client,
        quicknode_url: &str,
    ) -> Result<ChainTip, String> {
        // Get block count
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": [],
            "id": 1
        });

        let resp = http_client
            .post(quicknode_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("QuickNode request failed: {}", e))?;

        let data: Value = resp
            .json()
            .await
            .map_err(|e| format!("QuickNode response parse failed: {}", e))?;

        if let Some(err) = data.get("error").filter(|e| !e.is_null()) {
            return Err(format!("QuickNode error: {}", err));
        }

        let height = data["result"].as_u64().unwrap_or(0);

        // Get best block hash
        let body2 = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getbestblockhash",
            "params": [],
            "id": 2
        });

        let resp2 = http_client
            .post(quicknode_url)
            .json(&body2)
            .send()
            .await
            .map_err(|e| format!("QuickNode request failed: {}", e))?;

        let data2: Value = resp2
            .json()
            .await
            .map_err(|e| format!("QuickNode response parse failed: {}", e))?;

        let hash = data2["result"].as_str().unwrap_or("").to_string();

        Ok(ChainTip {
            height,
            hash,
            time: None,
        })
    }

    /// Get balance via QuickNode (derived from bb_getutxos)
    pub async fn get_balance_quicknode(
        http_client: &reqwest::Client,
        quicknode_url: &str,
        address: &str,
    ) -> Result<AddressBalance, String> {
        let utxos = Self::get_utxos_quicknode(http_client, quicknode_url, address).await?;
        let confirmed: u64 = utxos.iter().map(|u| u.value).sum();
        let utxo_count = utxos.len();

        Ok(AddressBalance {
            address: address.to_string(),
            confirmed,
            unconfirmed: 0,
            utxo_count,
        })
    }

    // ==================== Node RPC methods (testnet4) ====================

    /// GET /wallet/utxos/{address} — List unspent outputs via scantxoutset (testnet4 only)
    /// Retries with abort if a scan is already in progress
    pub async fn get_utxos(client: Arc<Client>, address: &str) -> Result<Vec<Utxo>, String> {
        let addr = address.to_string();

        tokio::task::spawn_blocking(move || {
            let descriptor = format!("addr({})", addr);
            let max_retries: u32 = 5;
            let mut last_error = String::new();

            for attempt in 0..max_retries {
                let scan_arg = bitcoincore_rpc::json::ScanTxOutRequest::Single(descriptor.clone());

                match client.scan_tx_out_set_blocking(&[scan_arg]) {
                    Ok(result) => {
                        let utxos = result
                            .unspents
                            .into_iter()
                            .map(|u| Utxo {
                                txid: u.txid.to_string(),
                                vout: u.vout,
                                value: u.amount.to_sat(),
                                script_pubkey: u.script_pub_key.to_string(),
                                confirmations: u.height as u32,
                            })
                            .collect();
                        return Ok(utxos);
                    }
                    Err(e) => {
                        last_error = format!("{}", e);
                        let is_scan_busy = last_error.contains("Scan already in progress");

                        // Only retry on "scan in progress" — connection errors fail fast
                        if is_scan_busy && attempt < max_retries - 1 {
                            tracing::warn!(
                                "scantxoutset: scan in progress (attempt {}/{}), aborting and retrying...",
                                attempt + 1, max_retries
                            );
                            let _: Result<Value, _> =
                                client.call("scantxoutset", &[serde_json::json!("abort")]);
                            std::thread::sleep(std::time::Duration::from_millis(
                                500 * (attempt as u64 + 1),
                            ));
                            continue;
                        }
                        return Err(format!("scantxoutset failed: {}", e));
                    }
                }
            }

            Err(format!("scantxoutset failed after {} retries: {}", max_retries, last_error))
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    /// GET /wallet/balance/{address} — Get balance for an address
    pub async fn get_balance(client: Arc<Client>, address: &str) -> Result<AddressBalance, String> {
        let utxos = Self::get_utxos(client, address).await?;
        let confirmed: u64 = utxos.iter().map(|u| u.value).sum();
        let utxo_count = utxos.len();

        Ok(AddressBalance {
            address: address.to_string(),
            confirmed,
            unconfirmed: 0, // scantxoutset only returns confirmed UTXOs
            utxo_count,
        })
    }

    /// GET /wallet/tx/{txid} — Get transaction details
    pub async fn get_transaction(
        client: Arc<Client>,
        txid: &str,
    ) -> Result<TransactionDetail, String> {
        let txid_str = txid.to_string();

        tokio::task::spawn_blocking(move || {
            let raw: Value = client
                .call(
                    "getrawtransaction",
                    &[serde_json::json!(txid_str), serde_json::json!(true)],
                )
                .map_err(|e| format!("getrawtransaction failed: {}", e))?;

            // Parse the verbose transaction response
            let inputs = raw["vin"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|vin| TxInput {
                    txid: vin["txid"].as_str().unwrap_or("").to_string(),
                    vout: vin["vout"].as_u64().unwrap_or(0) as u32,
                    script_sig: vin["scriptSig"]["hex"].as_str().unwrap_or("").to_string(),
                    sequence: vin["sequence"].as_u64().unwrap_or(0) as u32,
                    witness: vin["txinwitness"]
                        .as_array()
                        .map(|w| {
                            w.iter()
                                .map(|v| v.as_str().unwrap_or("").to_string())
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect();

            let outputs = raw["vout"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|vout| TxOutput {
                    value: vout["value"].as_f64().unwrap_or(0.0),
                    n: vout["n"].as_u64().unwrap_or(0) as u32,
                    script_pubkey: ScriptPubKey {
                        asm: vout["scriptPubKey"]["asm"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        hex: vout["scriptPubKey"]["hex"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        script_type: vout["scriptPubKey"]["type"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        address: vout["scriptPubKey"]["address"]
                            .as_str()
                            .map(|s| s.to_string()),
                    },
                })
                .collect();

            Ok(TransactionDetail {
                txid: raw["txid"].as_str().unwrap_or("").to_string(),
                version: raw["version"].as_i64().unwrap_or(0) as i32,
                locktime: raw["locktime"].as_u64().unwrap_or(0) as u32,
                size: raw["size"].as_u64().unwrap_or(0) as usize,
                vsize: raw["vsize"].as_u64().unwrap_or(0) as usize,
                weight: raw["weight"].as_u64().unwrap_or(0) as usize,
                fee: raw["fee"].as_f64(),
                confirmations: raw["confirmations"].as_u64().map(|c| c as u32),
                block_hash: raw["blockhash"].as_str().map(|s| s.to_string()),
                block_height: raw["blockheight"].as_u64().map(|h| h as u32),
                time: raw["time"].as_u64(),
                inputs,
                outputs,
                hex: raw["hex"].as_str().unwrap_or("").to_string(),
            })
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    /// POST /wallet/broadcast — Broadcast a signed transaction
    pub async fn broadcast_transaction(
        client: Arc<Client>,
        raw_tx_hex: &str,
    ) -> Result<BroadcastResult, String> {
        let hex = raw_tx_hex.to_string();

        tokio::task::spawn_blocking(move || {
            let txid: String = client
                .call("sendrawtransaction", &[serde_json::json!(hex)])
                .map_err(|e| format!("sendrawtransaction failed: {}", e))?;

            Ok(BroadcastResult { txid })
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    /// GET /wallet/fee-estimate — Get fee rate estimate
    pub async fn get_fee_estimate(
        client: Arc<Client>,
        target_blocks: Option<u16>,
    ) -> Result<FeeEstimate, String> {
        let blocks = target_blocks.unwrap_or(6);

        tokio::task::spawn_blocking(move || {
            let result: Value = client
                .call("estimatesmartfee", &[serde_json::json!(blocks)])
                .map_err(|e| format!("estimatesmartfee failed: {}", e))?;

            let fee_rate = result["feerate"].as_f64().unwrap_or(0.00001); // fallback minimum fee rate (BTC/kB)

            Ok(FeeEstimate { fee_rate, blocks })
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    /// GET /wallet/tip — Get current chain tip
    pub async fn get_chain_tip(client: Arc<Client>) -> Result<ChainTip, String> {
        tokio::task::spawn_blocking(move || {
            let block_count = client
                .get_block_count()
                .map_err(|e| format!("getblockcount failed: {}", e))?;

            let best_hash = client
                .get_best_block_hash()
                .map_err(|e| format!("getbestblockhash failed: {}", e))?;

            // Get block header for timestamp
            let header: Value = client
                .call(
                    "getblockheader",
                    &[serde_json::json!(best_hash.to_string())],
                )
                .map_err(|e| format!("getblockheader failed: {}", e))?;

            let time = header["time"].as_u64();

            Ok(ChainTip {
                height: block_count,
                hash: best_hash.to_string(),
                time,
            })
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }
}
