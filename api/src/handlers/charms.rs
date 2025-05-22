// Handlers for charm-related API endpoints

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::error::ExplorerResult;
use crate::handlers::AppState;
use crate::models::{
    CharmCountResponse, CharmData, CharmsResponse, GetCharmNumbersQuery, GetCharmsByTypeQuery,
};
use crate::services::charm_service;

/// Handler for GET /charms/count - Returns the count of charms, optionally filtered by asset type
pub async fn get_charm_numbers(
    State(state): State<AppState>,
    Query(params): Query<GetCharmNumbersQuery>,
) -> ExplorerResult<Json<CharmCountResponse>> {
    let asset_type = params.asset_type.as_deref();
    let response = charm_service::get_charm_numbers_by_type(&state, asset_type).await?;
    Ok(Json(response))
}

/// Handler for GET /charms - Returns all charms
pub async fn get_charms(State(state): State<AppState>) -> ExplorerResult<Json<CharmsResponse>> {
    let response = charm_service::get_all_charms(&state).await?;
    Ok(Json(response))
}

/// Handler for GET /charms/by-type - Returns charms filtered by asset type
pub async fn get_charms_by_type(
    State(state): State<AppState>,
    Query(params): Query<GetCharmsByTypeQuery>,
) -> ExplorerResult<Json<CharmsResponse>> {
    let response = charm_service::get_charms_by_type(&state, &params.asset_type).await?;
    Ok(Json(response))
}

/// Handler for GET /charms/{txid} - Returns a specific charm by its transaction ID
pub async fn get_charm_by_txid(
    State(state): State<AppState>,
    Path(txid): Path<String>,
) -> ExplorerResult<Json<CharmData>> {
    let charm_data = charm_service::get_charm_by_txid(&state, &txid).await?;
    Ok(Json(charm_data))
}

/// Handler for GET /charms/by-charmid/{charmid} - Returns a specific charm by its charm ID
pub async fn get_charm_by_charmid(
    State(state): State<AppState>,
    Path(charmid): Path<String>,
) -> ExplorerResult<Json<CharmData>> {
    let charm_data = charm_service::get_charm_by_charmid(&state, &charmid).await?;
    Ok(Json(charm_data))
}
