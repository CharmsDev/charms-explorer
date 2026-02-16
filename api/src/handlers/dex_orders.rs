// [RJJ-DEX] Handlers for DEX orders endpoints (Charms Cast integration)

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::error::ExplorerResult;
use crate::handlers::AppState;
use crate::services::dex_orders_service::{self, DexOrderResponse, DexOrdersListResponse};

#[derive(Debug, Deserialize)]
pub struct OpenOrdersQuery {
    pub asset: Option<String>,
    pub side: Option<String>,
    pub network: Option<String>,
}

/// GET /dex/orders/open?asset=...&side=...&network=...
/// Returns all active/open DEX positions
pub async fn get_open_orders(
    State(state): State<AppState>,
    Query(params): Query<OpenOrdersQuery>,
) -> ExplorerResult<Json<DexOrdersListResponse>> {
    let response = dex_orders_service::get_open_orders(
        &state,
        params.asset.as_deref(),
        params.side.as_deref(),
        params.network.as_deref(),
    )
    .await?;
    Ok(Json(response))
}

/// GET /dex/orders/{order_id}
/// Returns a single order by ID
pub async fn get_order_by_id(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> ExplorerResult<Json<Option<DexOrderResponse>>> {
    let response = dex_orders_service::get_order_by_id(&state, &order_id).await?;
    Ok(Json(response))
}

/// GET /dex/orders/by-asset/{asset_app_id}
/// Returns all orders (any status) for a specific asset
pub async fn get_orders_by_asset(
    State(state): State<AppState>,
    Path(asset_app_id): Path<String>,
) -> ExplorerResult<Json<DexOrdersListResponse>> {
    let response = dex_orders_service::get_orders_by_asset(&state, &asset_app_id).await?;
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct MakerOrdersQuery {
    pub status: Option<String>,
}

/// GET /dex/orders/by-maker/{maker}?status=open
/// Returns orders by maker address, optionally filtered by status
pub async fn get_orders_by_maker(
    State(state): State<AppState>,
    Path(maker): Path<String>,
    Query(params): Query<MakerOrdersQuery>,
) -> ExplorerResult<Json<DexOrdersListResponse>> {
    let response =
        dex_orders_service::get_orders_by_maker(&state, &maker, params.status.as_deref()).await?;
    Ok(Json(response))
}
