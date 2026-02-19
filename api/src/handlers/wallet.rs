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
/// QuickNode first (indexed, instant) → RPC fallback
pub async fn get_wallet_balance(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<NetworkQuery>,
) -> ExplorerResult<Json<serde_json::Value>> {
    let qn = quicknode_url(&state).to_string();

    let result = if !qn.is_empty() {
        match WalletService::get_balance_quicknode(&state.http_client, &qn, &address).await {
            Ok(balance) => Ok(balance),
            Err(e) => {
                tracing::warn!("Balance: QuickNode failed, falling back to RPC: {}", e);
                WalletService::get_balance(rpc_client(&state, &params.network), &address).await
            }
        }
    } else {
        WalletService::get_balance(rpc_client(&state, &params.network), &address).await
    };

    match result {
        Ok(balance) => Ok(Json(serde_json::json!(balance))),
        Err(e) => {
            tracing::error!("Wallet: failed to get balance for {}: {}", address, e);
            Err(ExplorerError::InternalError(e))
        }
    }
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
pub async fn get_wallet_charm_balances(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<NetworkQuery>,
) -> ExplorerResult<Json<serde_json::Value>> {
    let network = params.network.as_str();

    match state
        .repositories
        .charm
        .get_charm_balances_by_address(&address, network)
        .await
    {
        Ok(balances) => Ok(Json(serde_json::json!({
            "address": address,
            "network": network,
            "balances": balances,
            "count": balances.len(),
        }))),
        Err(e) => {
            tracing::error!(
                "Wallet: failed to get charm balances for {}: {:?}",
                address,
                e
            );
            Err(ExplorerError::InternalError(format!("{:?}", e)))
        }
    }
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
