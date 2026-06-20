// Esplora-compatible Bitcoin API client.
// mainnet/testnet via Maestro (https://docs.gomaestro.org/bitcoin/),
// testnet4 via mempool.space (Maestro has no testnet4 endpoint).
// Same JSON shape across providers — the only difference is the base URL
// and whether the api-key header is needed.

use serde_json::Value;

use super::wallet_service::{AddressTxRecord, ChainTip, FeeEstimate, Utxo};

/// Esplora-compatible base URL per network. The Maestro hosts already include
/// `/v0/esplora`, the mempool.space host includes `/<net>/api` — both produce
/// the same path layout for everything that follows.
fn base_url(network: &str) -> &'static str {
    match network {
        "testnet4" => "https://mempool.space/testnet4/api",
        "testnet" | "testnet3" => "https://xbt-testnet.gomaestro-api.org/v0/esplora",
        _ => "https://xbt-mainnet.gomaestro-api.org/v0/esplora",
    }
}

/// True for providers that require the `api-key` header (Maestro). The header
/// is harmless when sent to mempool.space too, but we skip the api_key check
/// upstream when the provider doesn't need it.
fn needs_api_key(network: &str) -> bool {
    !matches!(network, "testnet4")
}

/// Maestro-only indexed endpoint base (no esplora prefix). Used for the
/// >1000 UTXO fallback. mempool.space has no equivalent — callers must skip
/// the indexed path for testnet4.
fn indexed_base_url(network: &str) -> Option<&'static str> {
    match network {
        "testnet4" => None,
        "testnet" | "testnet3" => Some("https://xbt-testnet.gomaestro-api.org/v0"),
        _ => Some("https://xbt-mainnet.gomaestro-api.org/v0"),
    }
}

/// Get UTXOs for an address via the network-appropriate Esplora provider.
///
/// Strategy:
/// 1. Try esplora endpoint first (mempool-aware: excludes spent, includes unconfirmed)
/// 2. If esplora returns 400 (>1000 UTXOs, Maestro only), fall back to indexed endpoint
pub async fn get_utxos(
    http_client: &reqwest::Client,
    api_key: &str,
    network: &str,
    address: &str,
    min_value: Option<u64>,
    quicknode_url: Option<&str>,
) -> Result<Vec<Utxo>, String> {
    let url = format!("{}/address/{}/utxo", base_url(network), address);
    let mut req = http_client.get(&url);
    if needs_api_key(network) {
        req = req.header("api-key", api_key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Esplora request failed: {}", e))?;

    let status = resp.status();
    if status.is_success() {
        let utxos_raw: Vec<Value> = resp
            .json()
            .await
            .map_err(|e| format!("Esplora parse failed: {}", e))?;

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

    // If 400 (too many UTXOs):
    //   - Try QuickNode first (returns all in ~0.9s, no pagination)
    //   - Fall back to indexed + mempool (Maestro only; skipped for testnet4)
    let error_body = resp.text().await.unwrap_or_default();
    if status.as_u16() == 400 && error_body.contains("Too many") {
        if let Some(qn_url) = quicknode_url {
            if !qn_url.is_empty() {
                tracing::info!("Esplora >1000 UTXOs for {}, trying QuickNode + mempool", address);
                match super::wallet_service::WalletService::get_utxos_quicknode(
                    http_client, qn_url, address
                ).await {
                    Ok(mut utxos) => {
                        supplement_with_mempool(http_client, api_key, network, address, &mut utxos).await;
                        if let Some(min) = min_value {
                            utxos.retain(|u| u.value >= min);
                        }
                        return Ok(utxos);
                    }
                    Err(e) => {
                        tracing::warn!("QuickNode failed for {}, using indexed: {}", address, e);
                    }
                }
            }
        }

        if indexed_base_url(network).is_some() {
            tracing::info!("Using indexed Maestro for {} (slow path)", address);
            return get_utxos_indexed_with_mempool(http_client, api_key, network, address, min_value).await;
        }
        return Err(format!(
            "Esplora {}/{}: too many UTXOs and no indexed fallback for this network",
            status, address
        ));
    }

    Err(format!("Esplora error {}: {}", status, error_body))
}

/// Indexed endpoint with cursor pagination + mempool supplement. Maestro only.
async fn get_utxos_indexed_with_mempool(
    http_client: &reqwest::Client,
    api_key: &str,
    network: &str,
    address: &str,
    min_value: Option<u64>,
) -> Result<Vec<Utxo>, String> {
    let indexed = match indexed_base_url(network) {
        Some(u) => u,
        None => return Err("indexed endpoint not available for this network".into()),
    };
    let mut all_utxos = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut url = format!("{}/addresses/{}/utxos?count=100", indexed, address);
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
            let indexed_vout = u["vout"].as_u64().unwrap_or(0) as u32;

            let real_vout = if min_value.is_some() {
                resolve_vout_from_tx(
                    http_client, api_key, network, &txid_str, address, value
                ).await.unwrap_or(indexed_vout)
            } else {
                indexed_vout
            };

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

    supplement_with_mempool(http_client, api_key, network, address, &mut all_utxos).await;

    if let Some(min) = min_value {
        all_utxos.retain(|u| u.value >= min);
    }

    Ok(all_utxos)
}

/// Supplement a UTXO list with mempool state. Adds unconfirmed outputs and
/// removes UTXOs being spent by mempool txs.
async fn supplement_with_mempool(
    http_client: &reqwest::Client,
    api_key: &str,
    network: &str,
    address: &str,
    utxos: &mut Vec<Utxo>,
) {
    let mempool_url = format!("{}/address/{}/txs/mempool", base_url(network), address);
    let mut req = http_client.get(&mempool_url);
    if needs_api_key(network) {
        req = req.header("api-key", api_key);
    }
    let resp = match req.send().await {
        Ok(r) => r,
        Err(_) => return,
    };

    let mempool_txs: Vec<Value> = match resp.json().await {
        Ok(t) => t,
        Err(_) => return,
    };

    if mempool_txs.is_empty() {
        return;
    }

    let mut spent_outpoints: std::collections::HashSet<(String, u32)> =
        std::collections::HashSet::new();

    for tx in &mempool_txs {
        if let Some(vins) = tx["vin"].as_array() {
            for vin in vins {
                let prev_txid = vin["txid"].as_str().unwrap_or("");
                let prev_vout = vin["vout"].as_u64().unwrap_or(0) as u32;
                spent_outpoints.insert((prev_txid.to_string(), prev_vout));
            }
        }

        let txid = tx["txid"].as_str().unwrap_or("");
        if let Some(vouts) = tx["vout"].as_array() {
            for (idx, vout) in vouts.iter().enumerate() {
                if vout["scriptpubkey_address"].as_str() == Some(address) {
                    let value = vout["value"].as_u64().unwrap_or(0);
                    let n = idx as u32;
                    let exists = utxos.iter().any(|u| u.txid == txid && u.vout == n);
                    if !exists {
                        utxos.push(Utxo {
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

    if !spent_outpoints.is_empty() {
        utxos.retain(|u| !spent_outpoints.contains(&(u.txid.clone(), u.vout)));
    }
}

/// Resolve the real vout for a UTXO by looking up the TX via esplora.
/// Maestro indexed endpoint returns vout:0 for all UTXOs (known bug).
async fn resolve_vout_from_tx(
    http_client: &reqwest::Client,
    api_key: &str,
    network: &str,
    txid: &str,
    address: &str,
    value: u64,
) -> Result<u32, String> {
    let url = format!("{}/tx/{}", base_url(network), txid);
    let mut req = http_client.get(&url);
    if needs_api_key(network) {
        req = req.header("api-key", api_key);
    }
    let resp = req
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

/// Get chain tip via the network-appropriate Esplora provider.
pub async fn get_chain_tip(
    http_client: &reqwest::Client,
    api_key: &str,
    network: &str,
) -> Result<ChainTip, String> {
    let mut req = http_client.get(format!("{}/blocks/tip/height", base_url(network)));
    if needs_api_key(network) {
        req = req.header("api-key", api_key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Esplora tip height failed: {}", e))?;

    let height: u64 = resp
        .text()
        .await
        .map_err(|e| format!("Esplora tip height parse failed: {}", e))?
        .trim()
        .parse()
        .unwrap_or(0);

    let mut req2 = http_client.get(format!("{}/blocks/tip/hash", base_url(network)));
    if needs_api_key(network) {
        req2 = req2.header("api-key", api_key);
    }
    let resp2 = req2
        .send()
        .await
        .map_err(|e| format!("Esplora tip hash failed: {}", e))?;

    let hash = resp2
        .text()
        .await
        .map_err(|e| format!("Esplora tip hash parse failed: {}", e))?
        .trim()
        .to_string();

    Ok(ChainTip {
        height,
        hash,
        time: None,
    })
}

/// Get address transaction history (UTXOs + tx history) for seeding.
pub async fn get_address_info(
    http_client: &reqwest::Client,
    api_key: &str,
    network: &str,
    address: &str,
) -> Result<(Vec<Utxo>, Vec<AddressTxRecord>), String> {
    let utxos = get_utxos(http_client, api_key, network, address, None, None).await?;

    let mut all_txs: Vec<AddressTxRecord> = Vec::new();
    let mut after_txid: Option<String> = None;
    let max_pages = 10;

    for _ in 0..max_pages {
        let url = match &after_txid {
            Some(txid) => format!(
                "{}/address/{}/txs/chain/{}",
                base_url(network), address, txid
            ),
            None => format!("{}/address/{}/txs", base_url(network), address),
        };

        let mut req = http_client.get(&url);
        if needs_api_key(network) {
            req = req.header("api-key", api_key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("Esplora address txs failed: {}", e))?;

        let txs: Vec<Value> = resp
            .json()
            .await
            .map_err(|e| format!("Esplora address txs parse failed: {}", e))?;

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

        if txs.len() < 25 {
            break;
        }
        after_txid = txs.last().and_then(|t| t["txid"].as_str().map(|s| s.to_string()));
    }

    Ok((utxos, all_txs))
}

/// Get fee estimates via Esplora.
pub async fn get_fee_estimate(
    http_client: &reqwest::Client,
    api_key: &str,
    network: &str,
    target_blocks: u16,
) -> Result<FeeEstimate, String> {
    let mut req = http_client.get(format!("{}/fee-estimates", base_url(network)));
    if needs_api_key(network) {
        req = req.header("api-key", api_key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Esplora fee estimate failed: {}", e))?;

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Esplora fee estimate parse failed: {}", e))?;

    let key = target_blocks.to_string();
    let fee_rate_sat_vb = data
        .get(&key)
        .and_then(|v| v.as_f64())
        .or_else(|| {
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

    let fee_rate_btc_kb = fee_rate_sat_vb / 100_000.0;

    Ok(FeeEstimate {
        fee_rate: fee_rate_btc_kb,
        blocks: target_blocks,
    })
}

/// Broadcast a signed transaction.
pub async fn broadcast_transaction(
    http_client: &reqwest::Client,
    api_key: &str,
    network: &str,
    raw_tx_hex: &str,
) -> Result<String, String> {
    let mut req = http_client
        .post(format!("{}/tx", base_url(network)))
        .header("Content-Type", "text/plain")
        .body(raw_tx_hex.to_string());
    if needs_api_key(network) {
        req = req.header("api-key", api_key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Esplora broadcast failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Esplora broadcast response failed: {}", e))?;

    if status.is_success() {
        Ok(body.trim().to_string())
    } else {
        Err(format!("Esplora broadcast error {}: {}", status, body))
    }
}

/// Get raw transaction hex.
pub async fn get_tx_hex(
    http_client: &reqwest::Client,
    api_key: &str,
    network: &str,
    txid: &str,
) -> Result<String, String> {
    let url = format!("{}/tx/{}/hex", base_url(network), txid);
    let mut req = http_client.get(&url);
    if needs_api_key(network) {
        req = req.header("api-key", api_key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Esplora tx hex failed: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Esplora tx hex parse failed: {}", e))?;

    if status.is_success() {
        Ok(body.trim().to_string())
    } else {
        Err(format!("Esplora tx hex error {}: {}", status, body))
    }
}
