// Handlers for transaction-related API endpoints

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::error::ExplorerResult;
use crate::handlers::AppState;
use crate::models::{
    GetTransactionsQuery, PaginatedResponse, TransactionAsset, TransactionData,
    TransactionsResponse,
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

/// Handler for GET /transactions/:txid - Returns a single transaction by txid,
/// enriched with asset metadata from charms + assets tables when available.
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

    let model = match tx {
        Some(m) => m,
        None => {
            return Err(crate::error::ExplorerError::NotFound(format!(
                "Transaction {} not found",
                txid
            )));
        }
    };

    let mut data = TransactionData::from(model);

    // Enrich with asset metadata if this transaction has charms
    if let Ok(charms) = state.repositories.charm.get_by_txids(&[txid]).await {
        if !charms.is_empty() {
            let app_ids: Vec<String> = charms.iter().map(|c| c.app_id.clone()).collect();

            // Lookup asset metadata (name, symbol) from assets table
            let assets_meta = state
                .repositories
                .asset_repository
                .find_by_app_ids(app_ids)
                .await
                .unwrap_or_default();

            let meta_map: std::collections::HashMap<String, &crate::entity::assets::Model> =
                assets_meta.iter().map(|a| (a.app_id.clone(), a)).collect();

            data.assets = charms
                .iter()
                .map(|charm| {
                    let meta = meta_map.get(&charm.app_id);
                    TransactionAsset {
                        app_id: charm.app_id.clone(),
                        name: meta.and_then(|m| m.name.clone()),
                        symbol: meta.and_then(|m| m.symbol.clone()),
                        amount: charm.amount,
                        asset_type: charm.asset_type.clone(),
                        vout: charm.vout,
                        verified: charm.verified,
                    }
                })
                .collect();
        }
    }

    Ok(Json(data))
}
