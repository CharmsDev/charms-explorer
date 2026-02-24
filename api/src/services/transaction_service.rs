// Transaction service — business logic for /v1/transactions endpoint

use crate::error::ExplorerResult;
use crate::handlers::AppState;
use crate::models::{PaginatedResponse, PaginationMeta, PaginationParams, TransactionData, TransactionsResponse};

/// Get all transactions paginated (all networks)
pub async fn get_all_transactions_paginated(
    state: &AppState,
    pagination: &PaginationParams,
) -> ExplorerResult<PaginatedResponse<TransactionsResponse>> {
    let (txs, total) = match state.repositories.transactions.get_all_paginated(pagination).await {
        Ok(result) => result,
        Err(err) => {
            tracing::warn!("Database error in get_all_transactions_paginated: {:?}", err);
            return Ok(PaginatedResponse {
                data: TransactionsResponse { transactions: vec![] },
                pagination: PaginationMeta {
                    total: 0,
                    page: pagination.page,
                    limit: pagination.limit,
                    total_pages: 0,
                },
            });
        }
    };

    let total_pages = if total == 0 {
        0
    } else {
        (total + pagination.limit - 1) / pagination.limit
    };

    let transactions: Vec<TransactionData> = txs.into_iter().map(TransactionData::from).collect();

    Ok(PaginatedResponse {
        data: TransactionsResponse { transactions },
        pagination: PaginationMeta {
            total,
            page: pagination.page,
            limit: pagination.limit,
            total_pages,
        },
    })
}

/// Get transactions paginated, filtered by network
pub async fn get_all_transactions_paginated_by_network(
    state: &AppState,
    pagination: &PaginationParams,
    network: &str,
) -> ExplorerResult<PaginatedResponse<TransactionsResponse>> {
    let (txs, total) = match state
        .repositories
        .transactions
        .get_all_paginated_by_network(pagination, network)
        .await
    {
        Ok(result) => result,
        Err(err) => {
            tracing::warn!(
                "Database error in get_all_transactions_paginated_by_network: {:?}",
                err
            );
            return Ok(PaginatedResponse {
                data: TransactionsResponse { transactions: vec![] },
                pagination: PaginationMeta {
                    total: 0,
                    page: pagination.page,
                    limit: pagination.limit,
                    total_pages: 0,
                },
            });
        }
    };

    let total_pages = if total == 0 {
        0
    } else {
        (total + pagination.limit - 1) / pagination.limit
    };

    let transactions: Vec<TransactionData> = txs.into_iter().map(TransactionData::from).collect();

    Ok(PaginatedResponse {
        data: TransactionsResponse { transactions },
        pagination: PaginationMeta {
            total,
            page: pagination.page,
            limit: pagination.limit,
            total_pages,
        },
    })
}
