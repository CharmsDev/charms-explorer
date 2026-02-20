// Wallet API endpoint handlers
// Strategy: RPC node (3s timeout) → QuickNode fallback

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::time::Duration;
use tokio::time::timeout;

use bitcoincore_rpc::Client;
use std::sync::Arc;

use crate::error::{ExplorerError, ExplorerResult};
use crate::handlers::AppState;
use crate::services::address_monitor_service::AddressMonitorService;
use crate::services::wallet_service::WalletService;

const RPC_TIMEOUT: Duration = Duration::from_secs(3);

/// Select the shared RPC client for the given network
fn rpc_client(state: &AppState, network: &str) -> Arc<Client> {
    match network {
        "testnet4" => state.rpc_testnet4.clone(),
        _ => state.rpc_mainnet.clone(),
    }
}

/// QuickNode endpoint (empty string = not configured)
fn quicknode_url(state: &AppState) -> &str {
    &state.config.bitcoin_mainnet_quicknode_endpoint
}

/// Try an RPC future with timeout; on failure, try QuickNode fallback
async fn rpc_with_fallback<T, RpcFut, QnFut>(
    rpc_future: RpcFut,
    qn_future: QnFut,
    qn_url: &str,
    label: &str,
) -> Result<T, String>
where
    RpcFut: std::future::Future<Output = Result<T, String>>,
    QnFut: std::future::Future<Output = Result<T, String>>,
{
    match timeout(RPC_TIMEOUT, rpc_future).await {
        Ok(Ok(val)) => Ok(val),
        Ok(Err(e)) => {
            if !qn_url.is_empty() {
                tracing::warn!("{}: RPC failed, falling back to QuickNode: {}", label, e);
                qn_future.await
            } else {
                Err(e)
            }
        }
        Err(_) => {
            if !qn_url.is_empty() {
                tracing::warn!(
                    "{}: RPC timed out ({}s), falling back to QuickNode",
                    label,
                    RPC_TIMEOUT.as_secs()
                );
                qn_future.await
            } else {
                Err(format!(
                    "{}: RPC timed out after {}s",
                    label,
                    RPC_TIMEOUT.as_secs()
                ))
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NetworkQuery {
    #[serde(default = "default_network")]
    pub network: String,
}

#[derive(Debug, Deserialize)]
pub struct FeeEstimateQuery {
    pub blocks: Option<u16>,
    #[serde(default = "default_network")]
    pub network: String,
}

#[derive(Debug, Deserialize)]
pub struct BroadcastRequest {
    pub raw_tx: String,
}

fn default_network() -> String {
    "mainnet".to_string()
}

/// GET /wallet/utxos/{address}
/// QuickNode first (indexed, instant) → RPC fallback (scantxoutset is slow)
pub async fn get_wallet_utxos(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<NetworkQuery>,
) -> ExplorerResult<Json<serde_json::Value>> {
    let qn = quicknode_url(&state).to_string();

    let result = if !qn.is_empty() {
        match WalletService::get_utxos_quicknode(&state.http_client, &qn, &address).await {
            Ok(utxos) => Ok(utxos),
            Err(e) => {
                tracing::warn!("UTXOs: QuickNode failed, falling back to RPC: {}", e);
                WalletService::get_utxos(rpc_client(&state, &params.network), &address).await
            }
        }
    } else {
        WalletService::get_utxos(rpc_client(&state, &params.network), &address).await
    };

    match result {
        Ok(utxos) => Ok(Json(serde_json::json!({
            "address": address,
            "utxos": utxos,
            "count": utxos.len(),
        }))),
        Err(e) => {
            tracing::error!("Wallet: failed to get UTXOs for {}: {}", address, e);
            Err(ExplorerError::InternalError(e))
        }
    }
}

/// GET /wallet/balance/{address}
/// Unified balance endpoint: BTC (available/locked) + Charms in one response.
///
/// # On-demand address monitoring
///
/// The `monitored_addresses` table starts empty. Addresses enter the system
/// through two paths:
///
/// 1. **Charm detection (indexer)** — When the indexer processes a block and
///    detects a charm, it auto-registers the charm's address. These addresses
///    are monitored from the moment the charm is found, so their BTC UTXOs
///    are always kept up to date by the indexer.
///
/// 2. **First balance request (this endpoint)** — When a user queries an
///    address that is NOT yet in the system (e.g. an address that has never
///    held a charm), the API seeds its current UTXO set from an external
///    source (QuickNode / Mempool) and registers it in `monitored_addresses`.
///    From that point on, the indexer keeps the UTXO set current as new
///    blocks arrive.
///
/// This means:
/// - Charm-holding addresses are always monitored (the indexer handles them).
/// - Plain BTC addresses start being monitored on their first balance query.
/// - The `"monitored": false` field in the response indicates the address was
///   just seeded for the first time (subsequent calls will return `true`).
pub async fn get_wallet_balance(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<NetworkQuery>,
) -> ExplorerResult<Json<serde_json::Value>> {
    let network = params.network.as_str();
    let qn = quicknode_url(&state).to_string();

    // Step 1: Ensure address is monitored (seeds from QuickNode if needed)
    let monitored = AddressMonitorService::ensure_monitored(
        &state.repositories.monitored_addresses,
        &state.repositories.utxo,
        &state.http_client,
        &qn,
        &address,
        network,
    )
    .await
    .unwrap_or(false);

    // Step 2: Get UTXOs from our DB
    let utxo_rows = state
        .repositories
        .utxo
        .get_by_address(&address, network)
        .await
        .unwrap_or_default();

    // Step 3: Get unspent charms for this address
    let charms = state
        .repositories
        .charm
        .get_unspent_charms_by_address(&address, network)
        .await
        .unwrap_or_default();

    // Build a set of (txid, vout) that hold charms
    let charm_utxo_keys: std::collections::HashSet<(String, i32)> =
        charms.iter().map(|c| (c.txid.clone(), c.vout)).collect();

    // Step 4: Classify UTXOs as available (no charms) or locked (has charms)
    let mut available: u64 = 0;
    let mut locked: u64 = 0;
    let mut btc_utxos: Vec<serde_json::Value> = Vec::new();

    for row in &utxo_rows {
        let has_charms = charm_utxo_keys.contains(&(row.txid.clone(), row.vout));
        let value = row.value as u64;

        if has_charms {
            locked += value;
        } else {
            available += value;
        }

        btc_utxos.push(serde_json::json!({
            "txid": row.txid,
            "vout": row.vout,
            "value": row.value,
            "blockHeight": row.block_height,
            "hasCharms": has_charms,
        }));
    }

    let confirmed = available + locked;

    // Step 5: Build charm balances (same logic as get_wallet_charm_balances)
    let mut charm_balance_map: std::collections::HashMap<
        String,
        (String, i64, Vec<serde_json::Value>),
    > = std::collections::HashMap::new();

    // Look up symbols
    let app_ids: Vec<String> = charms.iter().map(|c| c.app_id.clone()).collect();
    let assets = state
        .repositories
        .asset_repository
        .find_by_app_ids(app_ids)
        .await
        .unwrap_or_default();
    let symbol_map: std::collections::HashMap<String, String> = assets
        .into_iter()
        .filter_map(|a| a.symbol.map(|s| (a.app_id, s)))
        .collect();

    for charm in &charms {
        let btc_value = utxo_rows
            .iter()
            .find(|r| r.txid == charm.txid && r.vout == charm.vout)
            .map(|r| r.value)
            .unwrap_or(546);

        let utxo_json = serde_json::json!({
            "txid": charm.txid,
            "vout": charm.vout,
            "value": btc_value,
            "amount": charm.amount,
            "confirmed": charm.block_height > 0,
            "blockHeight": charm.block_height,
        });

        let entry = charm_balance_map
            .entry(charm.app_id.clone())
            .or_insert_with(|| (charm.asset_type.clone(), 0, Vec::new()));

        entry.1 += charm.amount;
        entry.2.push(utxo_json);
    }

    let charm_balances: Vec<serde_json::Value> = charm_balance_map
        .into_iter()
        .map(|(app_id, (asset_type, total, utxos))| {
            let symbol = symbol_map.get(&app_id).cloned().unwrap_or_default();
            serde_json::json!({
                "appId": app_id,
                "assetType": asset_type,
                "symbol": symbol,
                "total": total,
                "utxos": utxos,
            })
        })
        .collect();

    // Step 6: Build unified response
    Ok(Json(serde_json::json!({
        "address": address,
        "network": network,
        "monitored": monitored,
        "btc": {
            "confirmed": confirmed,
            "unconfirmed": 0,
            "total": confirmed,
            "available": available,
            "locked": locked,
            "utxos": btc_utxos,
        },
        "charms": {
            "balances": charm_balances,
            "count": charm_balances.len(),
        },
    })))
}

/// GET /wallet/tx/{txid}
pub async fn get_wallet_transaction(
    State(state): State<AppState>,
    Path(txid): Path<String>,
    Query(params): Query<NetworkQuery>,
) -> ExplorerResult<Json<serde_json::Value>> {
    let client = rpc_client(&state, &params.network);

    match WalletService::get_transaction(client, &txid).await {
        Ok(tx) => Ok(Json(serde_json::json!(tx))),
        Err(e) => {
            tracing::error!("Wallet: failed to get transaction {}: {}", txid, e);
            Err(ExplorerError::NotFound(format!(
                "Transaction {} not found: {}",
                txid, e
            )))
        }
    }
}

/// POST /wallet/broadcast
pub async fn broadcast_wallet_transaction(
    State(state): State<AppState>,
    Query(params): Query<NetworkQuery>,
    Json(body): Json<BroadcastRequest>,
) -> ExplorerResult<Json<serde_json::Value>> {
    let client = rpc_client(&state, &params.network);

    match WalletService::broadcast_transaction(client, &body.raw_tx).await {
        Ok(result) => Ok(Json(serde_json::json!(result))),
        Err(e) => {
            tracing::error!("Wallet: failed to broadcast transaction: {}", e);
            Err(ExplorerError::InternalError(e))
        }
    }
}

/// GET /wallet/fee-estimate?blocks=6
pub async fn get_wallet_fee_estimate(
    State(state): State<AppState>,
    Query(params): Query<FeeEstimateQuery>,
) -> ExplorerResult<Json<serde_json::Value>> {
    let client = rpc_client(&state, &params.network);

    match WalletService::get_fee_estimate(client, params.blocks).await {
        Ok(estimate) => Ok(Json(serde_json::json!(estimate))),
        Err(e) => {
            tracing::error!("Wallet: failed to get fee estimate: {}", e);
            Err(ExplorerError::InternalError(e))
        }
    }
}

/// GET /wallet/charms/{address}
/// Returns confirmed + unconfirmed charm balances from the indexed DB (instant)
/// Response shape matches Cast's explorerApiProvider.getAggregateCharmBalances()
pub async fn get_wallet_charm_balances(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<NetworkQuery>,
) -> ExplorerResult<Json<serde_json::Value>> {
    let network = params.network.as_str();

    // 1. Get all unspent charms for this address
    let charms = match state
        .repositories
        .charm
        .get_unspent_charms_by_address(&address, network)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Wallet: failed to get charms for {}: {:?}", address, e);
            return Err(ExplorerError::InternalError(format!("{:?}", e)));
        }
    };

    if charms.is_empty() {
        return Ok(Json(serde_json::json!({
            "address": address,
            "network": network,
            "balances": [],
            "count": 0,
        })));
    }

    // 2. Get sibling app_ids per (txid, vout) — single efficient SQL query
    let utxo_app_ids = state
        .repositories
        .charm
        .get_sibling_app_ids_for_address(&address, network)
        .await
        .unwrap_or_default();

    // 3. Look up BTC values from address_utxos
    let btc_utxos = state
        .repositories
        .utxo
        .get_by_address(&address, network)
        .await
        .unwrap_or_default();
    let utxo_values: std::collections::HashMap<(String, i32), i64> = btc_utxos
        .iter()
        .map(|u| ((u.txid.clone(), u.vout), u.value))
        .collect();

    // 5. Look up symbols from assets table
    let app_ids: Vec<String> = charms.iter().map(|c| c.app_id.clone()).collect();
    let assets = state
        .repositories
        .asset_repository
        .find_by_app_ids(app_ids.clone())
        .await
        .unwrap_or_default();
    let symbol_map: std::collections::HashMap<String, String> = assets
        .into_iter()
        .filter_map(|a| a.symbol.map(|s| (a.app_id, s)))
        .collect();

    // 6. Group charms by app_id and build Cast-compatible response
    let mut balance_map: std::collections::HashMap<
        String,
        (String, String, i64, i64, Vec<serde_json::Value>),
    > = std::collections::HashMap::new();
    // key -> (asset_type, symbol, confirmed_total, unconfirmed_total, utxos_json)

    for charm in &charms {
        let confirmed = charm.block_height > 0;
        let key = (charm.txid.clone(), charm.vout);
        let all_app_ids = utxo_app_ids
            .get(&key)
            .cloned()
            .unwrap_or_else(|| vec![charm.app_id.clone()]);
        let has_order_charm = all_app_ids.iter().any(|id| id.starts_with("b/"));
        let btc_value = utxo_values.get(&key).copied().unwrap_or(546);
        let symbol = symbol_map.get(&charm.app_id).cloned().unwrap_or_default();

        let utxo_json = serde_json::json!({
            "txid": charm.txid,
            "vout": charm.vout,
            "value": btc_value,
            "address": address,
            "appId": charm.app_id,
            "amount": charm.amount,
            "confirmed": confirmed,
            "blockHeight": charm.block_height,
            "hasOrderCharm": has_order_charm,
            "allCharmAppIds": all_app_ids,
        });

        let entry = balance_map
            .entry(charm.app_id.clone())
            .or_insert_with(|| (charm.asset_type.clone(), symbol.clone(), 0, 0, Vec::new()));

        if confirmed {
            entry.2 += charm.amount;
        } else {
            entry.3 += charm.amount;
        }
        entry.4.push(utxo_json);
    }

    // 7. Build final balances array
    let balances: Vec<serde_json::Value> = balance_map
        .into_iter()
        .map(
            |(app_id, (asset_type, symbol, confirmed, unconfirmed, utxos))| {
                serde_json::json!({
                    "appId": app_id,
                    "assetType": asset_type,
                    "symbol": symbol,
                    "confirmed": confirmed,
                    "unconfirmed": unconfirmed,
                    "total": confirmed + unconfirmed,
                    "utxos": utxos,
                })
            },
        )
        .collect();

    Ok(Json(serde_json::json!({
        "address": address,
        "network": network,
        "balances": balances,
        "count": balances.len(),
    })))
}

/// GET /wallet/tip
pub async fn get_wallet_chain_tip(
    State(state): State<AppState>,
    Query(params): Query<NetworkQuery>,
) -> ExplorerResult<Json<serde_json::Value>> {
    let client = rpc_client(&state, &params.network);
    let qn = quicknode_url(&state).to_string();
    let http = state.http_client.clone();

    let result = rpc_with_fallback(
        WalletService::get_chain_tip(client),
        WalletService::get_chain_tip_quicknode(&http, &qn),
        &qn,
        "Tip",
    )
    .await;

    match result {
        Ok(tip) => Ok(Json(serde_json::json!(tip))),
        Err(e) => {
            tracing::error!("Wallet: failed to get chain tip: {}", e);
            Err(ExplorerError::InternalError(e))
        }
    }
}
