// Handlers for charm-related API endpoints

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::error::ExplorerResult;
use crate::handlers::AppState;
use crate::models::{
    CharmCountResponse, CharmData, CharmsResponse, GetCharmNumbersQuery, GetCharmsByTypeQuery,
    GetCharmsQuery, LikeCharmRequest, LikeResponse, PaginatedResponse,
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

/// Handler for GET /charms - Returns all charms with pagination, optionally filtered by network
pub async fn get_charms(
    State(state): State<AppState>,
    Query(params): Query<GetCharmsQuery>,
) -> ExplorerResult<Json<PaginatedResponse<CharmsResponse>>> {
    let response = if let Some(network) = &params.network {
        charm_service::get_all_charms_paginated_by_network(&state, &params.pagination, params.user_id, Some(network)).await?
    } else {
        charm_service::get_all_charms_paginated(&state, &params.pagination, params.user_id).await?
    };
    Ok(Json(response))
}

/// Handler for GET /charms/by-type - Returns charms filtered by asset type with pagination
pub async fn get_charms_by_type(
    State(state): State<AppState>,
    Query(params): Query<GetCharmsByTypeQuery>,
) -> ExplorerResult<Json<PaginatedResponse<CharmsResponse>>> {
    // Use default user_id of 1 as specified in requirements
    let response = charm_service::get_charms_by_type_paginated(&state, &params.asset_type, &params.pagination, 1).await?;
    Ok(Json(response))
}

/// Handler for GET /charms/{txid} - Returns a specific charm by its transaction ID
pub async fn get_charm_by_txid(
    State(state): State<AppState>,
    Path(txid): Path<String>,
) -> ExplorerResult<Json<CharmData>> {
    // Use default user_id of 1 as specified in requirements
    let charm_data = charm_service::get_charm_by_txid(&state, &txid, 1).await?;
    Ok(Json(charm_data))
}

/// Handler for GET /charms/by-charmid/{charmid} - Returns a specific charm by its charm ID
pub async fn get_charm_by_charmid(
    State(state): State<AppState>,
    Path(charmid): Path<String>,
) -> ExplorerResult<Json<CharmData>> {
    // Use default user_id of 1 as specified in requirements
    let charm_data = charm_service::get_charm_by_charmid(&state, &charmid, 1).await?;
    Ok(Json(charm_data))
}

/// Handler for POST /charms/like - Adds a like to a charm
pub async fn like_charm(
    State(state): State<AppState>,
    Json(request): Json<LikeCharmRequest>,
) -> ExplorerResult<Json<LikeResponse>> {
    let response = charm_service::add_like(&state, &request).await?;
    Ok(Json(response))
}

/// Handler for DELETE /charms/like - Removes a like from a charm
pub async fn unlike_charm(
    State(state): State<AppState>,
    Json(request): Json<LikeCharmRequest>,
) -> ExplorerResult<Json<LikeResponse>> {
    let response = charm_service::remove_like(&state, &request).await?;
    Ok(Json(response))
}

/// [RJJ-ADDRESS-SEARCH] Handler for GET /charms/by-address/{address}
/// Returns UNSPENT charms for a Bitcoin address
pub async fn get_charms_by_address(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<GetCharmsQuery>,
) -> ExplorerResult<Json<CharmsResponse>> {
    let response = charm_service::get_charms_by_address(&state, &address, params.user_id).await?;
    Ok(Json(response))
}
