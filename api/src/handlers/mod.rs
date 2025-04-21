// Handlers Module
// This module contains the API endpoint handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::{DbError, Repositories};

// Type alias for the application state
pub type AppState = Arc<Repositories>;

// Error response
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

// Convert DbError to an HTTP response
impl IntoResponse for DbError {
    fn into_response(self) -> Response {
        let status = match self {
            DbError::ConnectionError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DbError::QueryError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(ErrorResponse {
            error: self.to_string(),
        });

        (status, body).into_response()
    }
}

// Query parameters for the get_charm_numbers endpoint
#[derive(Debug, Deserialize)]
pub struct GetCharmNumbersQuery {
    #[serde(rename = "type")]
    asset_type: Option<String>,
}

// Response for the get_charm_count endpoint
#[derive(Debug, Serialize)]
pub struct CharmCountResponse {
    count: usize,
}

// Handler for GET /charms/count
pub async fn get_charm_numbers(
    State(state): State<AppState>,
    Query(params): Query<GetCharmNumbersQuery>,
) -> Result<Json<CharmCountResponse>, DbError> {
    let asset_type = params.asset_type.as_deref();
    let charm_numbers = state.charm.get_charm_numbers_by_type(asset_type).await?;
    let count = charm_numbers.len();

    Ok(Json(CharmCountResponse { count }))
}

// Response for the get_charms endpoint
#[derive(Debug, Serialize)]
pub struct CharmsResponse {
    charms: Vec<CharmData>,
}

// Charm data for the response
#[derive(Debug, Serialize)]
pub struct CharmData {
    txid: String,
    charmid: String,
    block_height: i32,
    data: serde_json::Value,
    date_created: String,
    asset_type: String,
}

// Handler for GET /api/charms
pub async fn get_charms(State(state): State<AppState>) -> Result<Json<CharmsResponse>, DbError> {
    let charms = state.charm.get_all().await?;

    let charm_data = charms
        .into_iter()
        .map(|charm| CharmData {
            txid: charm.txid,
            charmid: charm.charmid,
            block_height: charm.block_height,
            data: charm.data,
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
        })
        .collect();

    Ok(Json(CharmsResponse { charms: charm_data }))
}

// Query parameters for the get_charms_by_type endpoint
#[derive(Debug, Deserialize)]
pub struct GetCharmsByTypeQuery {
    #[serde(rename = "type")]
    asset_type: String,
}

// Handler for GET /api/charms/by-type
pub async fn get_charms_by_type(
    State(state): State<AppState>,
    Query(params): Query<GetCharmsByTypeQuery>,
) -> Result<Json<CharmsResponse>, DbError> {
    let charms = state.charm.find_by_asset_type(&params.asset_type).await?;

    let charm_data = charms
        .into_iter()
        .map(|charm| CharmData {
            txid: charm.txid,
            charmid: charm.charmid,
            block_height: charm.block_height,
            data: charm.data,
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
        })
        .collect();

    Ok(Json(CharmsResponse { charms: charm_data }))
}

// Handler for GET /api/charms/:txid
pub async fn get_charm_by_txid(
    State(state): State<AppState>,
    Path(txid): Path<String>,
) -> Result<Json<CharmData>, DbError> {
    let charm = state
        .charm
        .get_by_txid(&txid)
        .await?
        .ok_or_else(|| DbError::QueryError(format!("Charm with txid {} not found", txid)))?;

    let charm_data = CharmData {
        txid: charm.txid,
        charmid: charm.charmid,
        block_height: charm.block_height,
        data: charm.data,
        date_created: charm.date_created.to_string(),
        asset_type: charm.asset_type,
    };

    Ok(Json(charm_data))
}

// Health check response
#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    status: String,
}

// Handler for GET /api/health
pub async fn health_check() -> Json<HealthCheckResponse> {
    Json(HealthCheckResponse {
        status: "ok".to_string(),
    })
}
