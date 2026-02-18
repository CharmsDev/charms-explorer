// Wallet API endpoint handlers
// Phase 1: Direct Bitcoin node access for wallet extension

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use bitcoincore_rpc::Client;
use std::sync::Arc;

use crate::error::{ExplorerError, ExplorerResult};
use crate::handlers::AppState;
use crate::services::wallet_service::WalletService;

/// Select the shared RPC client for the given network
fn rpc_client(state: &AppState, network: &str) -> Arc<Client> {
    match network {
        "testnet4" => state.rpc_testnet4.clone(),
        _ => state.rpc_mainnet.clone(),
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

/// GET /wallet/utxos/{address} — List unspent outputs for an address
/// Mainnet: QuickNode (bb_getutxos, indexed, instant)
/// Testnet4: Node RPC (scantxoutset, small UTXO set)
pub async fn get_wallet_utxos(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<NetworkQuery>,
) -> ExplorerResult<Json<serde_json::Value>> {
    let result = if params.network != "testnet4"
        && !state.config.bitcoin_mainnet_quicknode_endpoint.is_empty()
    {
        // Mainnet: use QuickNode (rate-limited)
        let _permit = state
            .quicknode_semaphore
            .acquire()
            .await
            .map_err(|e| ExplorerError::InternalError(format!("Semaphore error: {}", e)))?;
        WalletService::get_utxos_quicknode(
            &state.http_client,
            &state.config.bitcoin_mainnet_quicknode_endpoint,
            &address,
        )
        .await
    } else {
        // Testnet4: use node RPC (scantxoutset)
        let client = rpc_client(&state, &params.network);
        let _permit = state
            .scan_semaphore
            .acquire()
            .await
            .map_err(|e| ExplorerError::InternalError(format!("Semaphore error: {}", e)))?;
        WalletService::get_utxos(client, &address).await
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

/// GET /wallet/balance/{address} — Get balance for an address
/// Mainnet: QuickNode | Testnet4: Node RPC
pub async fn get_wallet_balance(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<NetworkQuery>,
) -> ExplorerResult<Json<serde_json::Value>> {
    let result = if params.network != "testnet4"
        && !state.config.bitcoin_mainnet_quicknode_endpoint.is_empty()
    {
        let _permit = state
            .quicknode_semaphore
            .acquire()
            .await
            .map_err(|e| ExplorerError::InternalError(format!("Semaphore error: {}", e)))?;
        WalletService::get_balance_quicknode(
            &state.http_client,
            &state.config.bitcoin_mainnet_quicknode_endpoint,
            &address,
        )
        .await
    } else {
        let client = rpc_client(&state, &params.network);
        let _permit = state
            .scan_semaphore
            .acquire()
            .await
            .map_err(|e| ExplorerError::InternalError(format!("Semaphore error: {}", e)))?;
        WalletService::get_balance(client, &address).await
    };

    match result {
        Ok(balance) => Ok(Json(serde_json::json!(balance))),
        Err(e) => {
            tracing::error!("Wallet: failed to get balance for {}: {}", address, e);
            Err(ExplorerError::InternalError(e))
        }
    }
}

/// GET /wallet/tx/{txid} — Get transaction details
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

/// POST /wallet/broadcast — Broadcast a signed transaction
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

/// GET /wallet/fee-estimate?blocks=6 — Get fee rate estimate
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

/// GET /wallet/tip — Get current chain tip (height + hash)
pub async fn get_wallet_chain_tip(
    State(state): State<AppState>,
    Query(params): Query<NetworkQuery>,
) -> ExplorerResult<Json<serde_json::Value>> {
    let client = rpc_client(&state, &params.network);

    match WalletService::get_chain_tip(client).await {
        Ok(tip) => Ok(Json(serde_json::json!(tip))),
        Err(e) => {
            tracing::error!("Wallet: failed to get chain tip: {}", e);
            Err(ExplorerError::InternalError(e))
        }
    }
}
