// Handlers for Bitcoin transaction data (non-charm transactions)

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::error::ExplorerResult;
use crate::handlers::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct BitcoinTransaction {
    pub txid: String,
    pub version: i32,
    pub locktime: u32,
    pub size: usize,
    pub weight: usize,
    pub fee: Option<u64>,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub block_height: Option<u32>,
    pub confirmations: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxInput {
    pub txid: String,
    pub vout: u32,
    pub script_sig: String,
    pub sequence: u32,
    pub witness: Vec<String>,
    pub prev_out: Option<PrevOut>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrevOut {
    pub value: u64,
    pub script_pubkey: String,
    pub address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxOutput {
    pub value: u64,
    pub n: u32,
    pub script_pubkey: String,
    pub address: Option<String>,
}

/// Handler for GET /bitcoin/tx/{txid} - Returns Bitcoin transaction data from node
pub async fn get_bitcoin_transaction(
    State(state): State<AppState>,
    Path(txid): Path<String>,
) -> ExplorerResult<Json<BitcoinTransaction>> {
    // Get raw transaction from Bitcoin node
    let bitcoin_client = &state.repositories.bitcoin_client;

    // Parse transaction
    let tx_result = bitcoin_client.get_transaction_details(&txid).await;

    match tx_result {
        Ok(tx_data) => Ok(Json(tx_data)),
        Err(e) => {
            tracing::error!("Error fetching Bitcoin transaction {}: {}", txid, e);
            Err(crate::error::ExplorerError::NotFound(format!(
                "Transaction {} not found",
                txid
            )))
        }
    }
}
