// Maestro Bitcoin API provider
// Primary provider for UTXOs, chain tip, address transactions, fee estimates, and broadcast.
// Uses Esplora-compatible and indexed endpoints.
// Docs: https://docs.gomaestro.org/bitcoin/

use serde_json::Value;

use super::wallet_service::{AddressTxRecord, ChainTip, FeeEstimate, Utxo};

const BASE_URL: &str = "https://xbt-mainnet.gomaestro-api.org/v0";

/// Get UTXOs for an address via Maestro esplora endpoint.
/// Uses /esplora/address/{addr}/utxo which is mempool-aware:
///   - EXCLUDES UTXOs being spent by mempool txs
///   - INCLUDES new unconfirmed outputs from mempool txs
/// No pagination — returns all UTXOs in a single response.
pub async fn get_utxos(
    http_client: &reqwest::Client,
    api_key: &str,
    address: &str,
) -> Result<Vec<Utxo>, String> {
    let url = format!("{}/esplora/address/{}/utxo", BASE_URL, address);

    let resp = http_client
        .get(&url)
        .header("api-key", api_key)
        .send()
        .await
        .map_err(|e| format!("Maestro request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Maestro error {}: {}", status, body));
    }

    let utxos_raw: Vec<Value> = resp
        .json()
        .await
        .map_err(|e| format!("Maestro response parse failed: {}", e))?;

    let utxos = utxos_raw
        .iter()
        .map(|u| {
            let confirmed = u["status"]["confirmed"].as_bool().unwrap_or(false);
            let block_height = u["status"]["block_height"].as_u64().unwrap_or(0);
            // Use block_height as a proxy for confirmations (0 if unconfirmed)
            let confirmations = if confirmed { block_height as u32 } else { 0 };

            Utxo {
                txid: u["txid"].as_str().unwrap_or("").to_string(),
                vout: u["vout"].as_u64().unwrap_or(0) as u32,
                value: u["value"].as_u64().unwrap_or(0),
                script_pubkey: String::new(), // esplora doesn't return this
                confirmations,
            }
        })
        .collect();

    Ok(utxos)
}

/// Get chain tip via Maestro esplora endpoints
pub async fn get_chain_tip(
    http_client: &reqwest::Client,
    api_key: &str,
) -> Result<ChainTip, String> {
    // Get block height
    let resp = http_client
        .get(&format!("{}/esplora/blocks/tip/height", BASE_URL))
        .header("api-key", api_key)
        .send()
        .await
        .map_err(|e| format!("Maestro tip height failed: {}", e))?;

    let height: u64 = resp
        .text()
        .await
        .map_err(|e| format!("Maestro tip height parse failed: {}", e))?
        .trim()
        .parse()
        .unwrap_or(0);

    // Get block hash
    let resp2 = http_client
        .get(&format!("{}/esplora/blocks/tip/hash", BASE_URL))
        .header("api-key", api_key)
        .send()
        .await
        .map_err(|e| format!("Maestro tip hash failed: {}", e))?;

    let hash = resp2
        .text()
        .await
        .map_err(|e| format!("Maestro tip hash parse failed: {}", e))?
        .trim()
        .to_string();

    Ok(ChainTip {
        height,
        hash,
        time: None,
    })
}

/// Get address transaction history via Maestro esplora endpoint
/// Returns (utxos, transactions) for seeding — same shape as QuickNode's get_address_quicknode
pub async fn get_address_info(
    http_client: &reqwest::Client,
    api_key: &str,
    address: &str,
) -> Result<(Vec<Utxo>, Vec<AddressTxRecord>), String> {
    // Get UTXOs
    let utxos = get_utxos(http_client, api_key, address).await?;

    // Get transaction history via esplora (paginated with after_txid)
    let mut all_txs: Vec<AddressTxRecord> = Vec::new();
    let mut after_txid: Option<String> = None;
    let max_pages = 10; // Safety cap: same as QuickNode

    for _ in 0..max_pages {
        let url = match &after_txid {
            Some(txid) => format!(
                "{}/esplora/address/{}/txs/chain/{}",
                BASE_URL, address, txid
            ),
            None => format!("{}/esplora/address/{}/txs", BASE_URL, address),
        };

        let resp = http_client
            .get(&url)
            .header("api-key", api_key)
            .send()
            .await
            .map_err(|e| format!("Maestro address txs failed: {}", e))?;

        let txs: Vec<Value> = resp
            .json()
            .await
            .map_err(|e| format!("Maestro address txs parse failed: {}", e))?;

        if txs.is_empty() {
            break;
        }

        for tx in &txs {
            let txid = tx["txid"].as_str().unwrap_or("").to_string();
            let status = &tx["status"];
            let block_height = status["block_height"].as_i64().map(|h| h as i32);
            let block_time = status["block_time"].as_i64();
            let confirmed = status["confirmed"].as_bool().unwrap_or(false);
            let fee = tx["fee"].as_i64().unwrap_or(0);

            // Calculate direction and amount from vin/vout
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

            all_txs.push(AddressTxRecord {
                txid,
                direction,
                amount,
                fee,
                block_height,
                block_time,
                confirmations: if confirmed { 1 } else { 0 },
            });
        }

        // Esplora pagination: use last txid as cursor for confirmed txs
        if txs.len() < 25 {
            break; // Last page
        }
        after_txid = txs.last().and_then(|t| t["txid"].as_str().map(|s| s.to_string()));
    }

    Ok((utxos, all_txs))
}

/// Get fee estimates via Maestro esplora endpoint
pub async fn get_fee_estimate(
    http_client: &reqwest::Client,
    api_key: &str,
    target_blocks: u16,
) -> Result<FeeEstimate, String> {
    let resp = http_client
        .get(&format!("{}/esplora/fee-estimates", BASE_URL))
        .header("api-key", api_key)
        .send()
        .await
        .map_err(|e| format!("Maestro fee estimate failed: {}", e))?;

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Maestro fee estimate parse failed: {}", e))?;

    // Fee estimates are keyed by target blocks: {"1": 5.0, "2": 4.5, ...}
    let key = target_blocks.to_string();
    let fee_rate_sat_vb = data
        .get(&key)
        .and_then(|v| v.as_f64())
        .or_else(|| {
            // Fallback: find the closest target
            data.as_object().and_then(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        k.parse::<u16>().ok().map(|n| (n, v.as_f64().unwrap_or(0.0)))
                    })
                    .min_by_key(|(n, _)| (*n as i32 - target_blocks as i32).unsigned_abs())
                    .map(|(_, rate)| rate)
            })
        })
        .unwrap_or(1.0);

    // Maestro returns sat/vB; convert to BTC/kB for compatibility with RPC format
    let fee_rate_btc_kb = fee_rate_sat_vb / 100_000.0;

    Ok(FeeEstimate {
        fee_rate: fee_rate_btc_kb,
        blocks: target_blocks,
    })
}

/// Broadcast a signed transaction via Maestro esplora endpoint
pub async fn broadcast_transaction(
    http_client: &reqwest::Client,
    api_key: &str,
    raw_tx_hex: &str,
) -> Result<String, String> {
    let resp = http_client
        .post(&format!("{}/esplora/tx", BASE_URL))
        .header("api-key", api_key)
        .header("Content-Type", "text/plain")
        .body(raw_tx_hex.to_string())
        .send()
        .await
        .map_err(|e| format!("Maestro broadcast failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Maestro broadcast response failed: {}", e))?;

    if status.is_success() {
        Ok(body.trim().to_string()) // Returns txid
    } else {
        Err(format!("Maestro broadcast error {}: {}", status, body))
    }
}
