// Maestro Bitcoin API provider
// Primary provider for UTXOs, chain tip, address transactions, fee estimates, and broadcast.
// Uses Esplora-compatible and indexed endpoints.
// Docs: https://docs.gomaestro.org/bitcoin/

use serde_json::Value;

use super::wallet_service::{AddressTxRecord, ChainTip, FeeEstimate, Utxo};

const BASE_URL: &str = "https://xbt-mainnet.gomaestro-api.org/v0";

/// Get UTXOs for an address via Maestro.
///
/// Strategy:
/// 1. Try esplora endpoint first (mempool-aware: excludes spent, includes unconfirmed)
/// 2. If esplora returns 400 (>1000 UTXOs), fall back to indexed endpoint with pagination
///
/// The esplora endpoint is preferred because it reflects real-time mempool state.
/// The indexed endpoint is used as fallback for addresses with many UTXOs (>1000).
pub async fn get_utxos(
    http_client: &reqwest::Client,
    api_key: &str,
    address: &str,
    min_value: Option<u64>,
) -> Result<Vec<Utxo>, String> {
    // Try esplora first (mempool-aware, no pagination)
    let url = format!("{}/esplora/address/{}/utxo", BASE_URL, address);
    let resp = http_client
        .get(&url)
        .header("api-key", api_key)
        .send()
        .await
        .map_err(|e| format!("Maestro request failed: {}", e))?;

    let status = resp.status();
    if status.is_success() {
        let utxos_raw: Vec<Value> = resp
            .json()
            .await
            .map_err(|e| format!("Maestro esplora parse failed: {}", e))?;

        let utxos = utxos_raw
            .iter()
            .filter_map(|u| {
                let value = u["value"].as_u64().unwrap_or(0);
                if let Some(min) = min_value {
                    if value < min { return None; }
                }
                let confirmed = u["status"]["confirmed"].as_bool().unwrap_or(false);
                let block_height = u["status"]["block_height"].as_u64().unwrap_or(0);
                let confirmations = if confirmed { block_height as u32 } else { 0 };
                Some(Utxo {
                    txid: u["txid"].as_str().unwrap_or("").to_string(),
                    vout: u["vout"].as_u64().unwrap_or(0) as u32,
                    value,
                    script_pubkey: String::new(),
                    confirmations,
                })
            })
            .collect();
        return Ok(utxos);
    }

    // If 400 (too many UTXOs), fall back to indexed endpoint with mempool supplement
    let error_body = resp.text().await.unwrap_or_default();
    if status.as_u16() == 400 && error_body.contains("Too many") {
        tracing::info!(
            "Maestro esplora >1000 UTXOs for {}, using indexed + mempool",
            address
        );
        return get_utxos_indexed_with_mempool(http_client, api_key, address, min_value).await;
    }

    Err(format!("Maestro error {}: {}", status, error_body))
}

/// Indexed endpoint with cursor pagination + mempool supplement for >1000 UTXO addresses.
/// ~400ms per page × N pages (sequential, cursor-dependent).
/// After fetching all confirmed UTXOs, supplements with mempool data to:
///   - Add unconfirmed outputs (new UTXOs from pending txs)
///   - Remove UTXOs being spent by pending txs
async fn get_utxos_indexed_with_mempool(
    http_client: &reqwest::Client,
    api_key: &str,
    address: &str,
    min_value: Option<u64>,
) -> Result<Vec<Utxo>, String> {
    let mut all_utxos = Vec::new();
    let mut cursor: Option<String> = None;

    // 1. Fetch all confirmed UTXOs via indexed endpoint (paginated, max 100/page)
    loop {
        let mut url = format!("{}/addresses/{}/utxos?count=100", BASE_URL, address);
        if let Some(ref c) = cursor {
            url.push_str(&format!("&cursor={}", c));
        }

        let resp = http_client
            .get(&url)
            .header("api-key", api_key)
            .send()
            .await
            .map_err(|e| format!("Maestro indexed request failed: {}", e))?;

        let status = resp.status();
        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("Maestro indexed parse failed: {}", e))?;

        if !status.is_success() {
            return Err(format!("Maestro indexed error {}: {}", status, body));
        }

        let empty = vec![];
        let data = body["data"].as_array().unwrap_or(&empty);
        for u in data {
            let value: u64 = u["satoshis"]
                .as_str()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
            if let Some(min) = min_value {
                if value < min { continue; }
            }
            let txid_str = u["txid"].as_str().unwrap_or("").to_string();

            // Maestro indexed returns vout:0 for all UTXOs (known bug).
            // Resolve the real vout by looking up the TX via esplora.
            let real_vout = resolve_vout_from_tx(
                http_client, api_key, &txid_str, address, value
            ).await.unwrap_or(u["vout"].as_u64().unwrap_or(0) as u32);

            all_utxos.push(Utxo {
                txid: txid_str,
                vout: real_vout,
                value,
                script_pubkey: String::new(),
                confirmations: u["confirmations"].as_u64().unwrap_or(0) as u32,
            });
        }

        cursor = body["next_cursor"].as_str().map(|s| s.to_string());
        if cursor.is_none() || data.is_empty() {
            break;
        }
    }

    // 2. Supplement with mempool state
    let mempool_url = format!("{}/esplora/address/{}/txs/mempool", BASE_URL, address);
    if let Ok(resp) = http_client
        .get(&mempool_url)
        .header("api-key", api_key)
        .send()
        .await
    {
        if let Ok(mempool_txs) = resp.json::<Vec<Value>>().await {
            let mut spent_outpoints: std::collections::HashSet<(String, u32)> =
                std::collections::HashSet::new();

            for tx in &mempool_txs {
                // Track inputs being spent
                if let Some(vins) = tx["vin"].as_array() {
                    for vin in vins {
                        let prev_txid = vin["txid"].as_str().unwrap_or("");
                        let prev_vout = vin["vout"].as_u64().unwrap_or(0) as u32;
                        spent_outpoints.insert((prev_txid.to_string(), prev_vout));
                    }
                }

                // Add unconfirmed outputs to this address
                let txid = tx["txid"].as_str().unwrap_or("");
                if let Some(vouts) = tx["vout"].as_array() {
                    for vout in vouts {
                        if vout["scriptpubkey_address"].as_str() == Some(address) {
                            let value = vout["value"].as_u64().unwrap_or(0);
                            if let Some(min) = min_value {
                                if value < min { continue; }
                            }
                            let n = vout["n"].as_u64().unwrap_or(0) as u32;
                            let exists = all_utxos.iter().any(|u| u.txid == txid && u.vout == n);
                            if !exists {
                                all_utxos.push(Utxo {
                                    txid: txid.to_string(),
                                    vout: n,
                                    value,
                                    script_pubkey: String::new(),
                                    confirmations: 0,
                                });
                            }
                        }
                    }
                }
            }

            // Remove UTXOs being spent by mempool txs
            if !spent_outpoints.is_empty() {
                all_utxos.retain(|u| !spent_outpoints.contains(&(u.txid.clone(), u.vout)));
            }
        }
    }

    Ok(all_utxos)
}

/// Resolve the real vout for a UTXO by looking up the TX via esplora.
/// Maestro indexed endpoint returns vout:0 for all UTXOs (known bug).
/// Matches by address + value to find the correct output index.
async fn resolve_vout_from_tx(
    http_client: &reqwest::Client,
    api_key: &str,
    txid: &str,
    address: &str,
    value: u64,
) -> Result<u32, String> {
    let url = format!("{}/esplora/tx/{}", BASE_URL, txid);
    let resp = http_client
        .get(&url)
        .header("api-key", api_key)
        .send()
        .await
        .map_err(|e| format!("resolve_vout failed: {}", e))?;

    if !resp.status().is_success() {
        return Err("TX lookup failed".to_string());
    }

    let tx: Value = resp.json().await.map_err(|e| format!("parse failed: {}", e))?;
    if let Some(vouts) = tx["vout"].as_array() {
        for (i, vout) in vouts.iter().enumerate() {
            let out_addr = vout["scriptpubkey_address"].as_str().unwrap_or("");
            let out_value = vout["value"].as_u64().unwrap_or(0);
            if out_addr == address && out_value == value {
                return Ok(i as u32);
            }
        }
    }

    Err("vout not found".to_string())
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
    let utxos = get_utxos(http_client, api_key, address, None).await?;

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
