// [RJJ-DEX] DEX orders service - Business logic for Charms Cast DEX positions

use crate::error::ExplorerResult;
use crate::handlers::AppState;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DexOrderResponse {
    pub order_id: String,
    pub txid: String,
    pub vout: i32,
    pub block_height: Option<i32>,
    pub platform: String,
    pub maker: String,
    pub side: String,
    pub exec_type: String,
    pub price_num: i64,
    pub price_den: i64,
    pub price_per_token: f64,
    pub amount: i64,
    pub quantity: i64,
    pub filled_amount: i64,
    pub filled_quantity: i64,
    pub asset_app_id: String,
    pub scrolls_address: Option<String>,
    pub status: String,
    pub confirmed: bool,
    pub parent_order_id: Option<String>,
    pub created_at: String,
    pub network: String,
}

#[derive(Debug, Serialize)]
pub struct DexOrdersListResponse {
    pub total: usize,
    pub orders: Vec<DexOrderResponse>,
}

fn model_to_response(m: &crate::entity::dex_orders::Model) -> DexOrderResponse {
    let price_per_token = if m.price_den != 0 {
        (m.price_num as f64 / m.price_den as f64) * 100_000_000.0
    } else {
        0.0
    };

    DexOrderResponse {
        order_id: m.order_id.clone(),
        txid: m.txid.clone(),
        vout: m.vout,
        block_height: m.block_height,
        platform: m.platform.clone(),
        maker: m.maker.clone(),
        side: m.side.clone(),
        exec_type: m.exec_type.clone(),
        price_num: m.price_num,
        price_den: m.price_den,
        price_per_token,
        amount: m.amount,
        quantity: m.quantity,
        filled_amount: m.filled_amount,
        filled_quantity: m.filled_quantity,
        asset_app_id: m.asset_app_id.clone(),
        scrolls_address: m.scrolls_address.clone(),
        status: m.status.clone(),
        confirmed: m.block_height.map_or(false, |h| h > 0),
        parent_order_id: m.parent_order_id.clone(),
        created_at: m.created_at.to_string(),
        network: m.network.clone(),
    }
}

/// Get all open/active DEX orders, optionally filtered
pub async fn get_open_orders(
    state: &AppState,
    asset_app_id: Option<&str>,
    side: Option<&str>,
    network: Option<&str>,
) -> ExplorerResult<DexOrdersListResponse> {
    let orders = state
        .repositories
        .dex_orders
        .find_open_orders(asset_app_id, side, network)
        .await
        .map_err(|e| {
            tracing::warn!("Database error in get_open_orders: {:?}", e);
            crate::error::ExplorerError::InternalError(format!("Database error: {}", e))
        })?;

    let responses: Vec<DexOrderResponse> = orders.iter().map(model_to_response).collect();

    Ok(DexOrdersListResponse {
        total: responses.len(),
        orders: responses,
    })
}

/// Get a single order by ID
pub async fn get_order_by_id(
    state: &AppState,
    order_id: &str,
) -> ExplorerResult<Option<DexOrderResponse>> {
    let order = state
        .repositories
        .dex_orders
        .get_by_id(order_id)
        .await
        .map_err(|e| {
            tracing::warn!("Database error in get_order_by_id: {:?}", e);
            crate::error::ExplorerError::InternalError(format!("Database error: {}", e))
        })?;

    Ok(order.as_ref().map(model_to_response))
}

/// Get all orders for a specific asset (any status)
pub async fn get_orders_by_asset(
    state: &AppState,
    asset_app_id: &str,
) -> ExplorerResult<DexOrdersListResponse> {
    let orders = state
        .repositories
        .dex_orders
        .find_by_asset(asset_app_id)
        .await
        .map_err(|e| {
            tracing::warn!("Database error in get_orders_by_asset: {:?}", e);
            crate::error::ExplorerError::InternalError(format!("Database error: {}", e))
        })?;

    let responses: Vec<DexOrderResponse> = orders.iter().map(model_to_response).collect();

    Ok(DexOrdersListResponse {
        total: responses.len(),
        orders: responses,
    })
}

/// Get orders by maker address
pub async fn get_orders_by_maker(
    state: &AppState,
    maker: &str,
    status: Option<&str>,
) -> ExplorerResult<DexOrdersListResponse> {
    let orders = state
        .repositories
        .dex_orders
        .find_by_maker(maker, status)
        .await
        .map_err(|e| {
            tracing::warn!("Database error in get_orders_by_maker: {:?}", e);
            crate::error::ExplorerError::InternalError(format!("Database error: {}", e))
        })?;

    let responses: Vec<DexOrderResponse> = orders.iter().map(model_to_response).collect();

    Ok(DexOrdersListResponse {
        total: responses.len(),
        orders: responses,
    })
}
