// [RJJ-STATS-HOLDERS] Stats holders service - Business logic for holder statistics

use crate::error::ExplorerResult;
use crate::handlers::AppState;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HolderInfo {
    pub address: String,
    pub total_amount: i64,
    pub charm_count: i32,
    pub percentage: f64,
    pub first_seen_block: i32,
    pub last_updated_block: i32,
}

#[derive(Debug, Serialize)]
pub struct HoldersResponse {
    pub app_id: String,
    pub total_holders: usize,
    pub total_supply: i64,
    pub holders: Vec<HolderInfo>,
}

/// Get holders for a specific asset (app_id)
pub async fn get_holders_by_app_id(
    state: &AppState,
    app_id: &str,
) -> ExplorerResult<HoldersResponse> {
    // [RJJ-TOKEN-METADATA] Convert token app_id (t/) to NFT app_id (n/) for lookup
    // Stats are consolidated under NFT app_ids in the database
    let lookup_app_id = if app_id.starts_with("t/") {
        app_id.replacen("t/", "n/", 1)
    } else {
        app_id.to_string()
    };

    // Remove the :N suffix for broader matching (stats are per-asset, not per-output)
    let base_app_id = if let Some(pos) = lookup_app_id.rfind(':') {
        &lookup_app_id[..pos]
    } else {
        &lookup_app_id
    };

    // Get holders from database
    let holders = match state
        .repositories
        .stats_holders
        .get_holders_by_app_id(base_app_id)
        .await
    {
        Ok(result) => result,
        Err(err) => {
            tracing::warn!("Database error in get_holders_by_app_id: {:?}", err);
            return Ok(HoldersResponse {
                app_id: app_id.to_string(),
                total_holders: 0,
                total_supply: 0,
                holders: vec![],
            });
        }
    };

    // Calculate total supply
    let total_supply: i64 = holders.iter().map(|h| h.total_amount).sum();

    // Transform to response format with percentages
    let holder_infos: Vec<HolderInfo> = holders
        .into_iter()
        .map(|h| {
            let percentage = if total_supply > 0 {
                (h.total_amount as f64 / total_supply as f64) * 100.0
            } else {
                0.0
            };

            HolderInfo {
                address: h.address,
                total_amount: h.total_amount,
                charm_count: h.charm_count,
                percentage,
                first_seen_block: h.first_seen_block,
                last_updated_block: h.last_updated_block,
            }
        })
        .collect();

    Ok(HoldersResponse {
        app_id: app_id.to_string(),
        total_holders: holder_infos.len(),
        total_supply,
        holders: holder_infos,
    })
}
