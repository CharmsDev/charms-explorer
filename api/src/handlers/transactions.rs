// Handlers for transaction-related API endpoints

use axum::{
    extract::{Query, State},
    Json,
};

use crate::error::ExplorerResult;
use crate::handlers::AppState;
use crate::models::{GetTransactionsQuery, PaginatedResponse, TransactionsResponse};
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
