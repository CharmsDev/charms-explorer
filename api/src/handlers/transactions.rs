// Handlers for transaction-related API endpoints

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::error::ExplorerResult;
use crate::handlers::AppState;
use crate::models::{
    GetTransactionsQuery, PaginatedResponse, TransactionData, TransactionsResponse,
};
use crate::services::transaction_service;

/// Handler for GET /transactions - Returns all transactions with pagination
pub async fn get_transactions(
    State(state): State<AppState>,
    Query(params): Query<GetTransactionsQuery>,
) -> ExplorerResult<Json<PaginatedResponse<TransactionsResponse>>> {
    let response = if let Some(network) = &params.network {
        transaction_service::get_all_transactions_paginated_by_network(
            &state,
            &params.pagination,
            network,
        )
        .await?
    } else {
        transaction_service::get_all_transactions_paginated(&state, &params.pagination).await?
    };
    Ok(Json(response))
}

/// Handler for GET /transactions/:txid - Returns a single transaction by txid
pub async fn get_transaction_by_txid(
    State(state): State<AppState>,
    Path(txid): Path<String>,
) -> ExplorerResult<Json<TransactionData>> {
    let tx = state
        .repositories
        .transactions
        .get_by_txid(&txid)
        .await
        .map_err(|e| {
            tracing::warn!("Database error in get_transaction_by_txid: {:?}", e);
            crate::error::ExplorerError::NotFound(format!("Transaction {} not found", txid))
        })?;

    match tx {
        Some(model) => Ok(Json(TransactionData::from(model))),
        None => Err(crate::error::ExplorerError::NotFound(format!(
            "Transaction {} not found",
            txid
        ))),
    }
}
