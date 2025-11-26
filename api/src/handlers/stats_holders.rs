// [RJJ-STATS-HOLDERS] Handlers for holder statistics endpoints

use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::ExplorerResult;
use crate::handlers::AppState;
use crate::services::stats_holders_service::{self, HoldersResponse};

/// [RJJ-STATS-HOLDERS] Handler for GET /assets/{app_id}/holders
/// Returns holder statistics for a specific asset
pub async fn get_asset_holders(
    State(state): State<AppState>,
    Path(app_id): Path<String>,
) -> ExplorerResult<Json<HoldersResponse>> {
    let response = stats_holders_service::get_holders_by_app_id(&state, &app_id).await?;
    Ok(Json(response))
}
