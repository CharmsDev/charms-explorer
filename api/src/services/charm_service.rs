// Charm-related business logic implementation

use crate::db::DbError;
use crate::error::ExplorerResult;
use crate::handlers::AppState;
use crate::models::{CharmCountResponse, CharmData, CharmsResponse};

pub async fn get_charm_numbers_by_type(
    state: &AppState,
    asset_type: Option<&str>,
) -> ExplorerResult<CharmCountResponse> {
    let charm_numbers = state.charm.get_charm_numbers_by_type(asset_type).await?;
    Ok(CharmCountResponse {
        count: charm_numbers.len(),
    })
}

pub async fn get_all_charms(state: &AppState) -> ExplorerResult<CharmsResponse> {
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
    Ok(CharmsResponse { charms: charm_data })
}

pub async fn get_charms_by_type(
    state: &AppState,
    asset_type: &str,
) -> ExplorerResult<CharmsResponse> {
    let charms = state.charm.find_by_asset_type(asset_type).await?;
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
    Ok(CharmsResponse { charms: charm_data })
}

pub async fn get_charm_by_txid(state: &AppState, txid: &str) -> ExplorerResult<CharmData> {
    let charm = state
        .charm
        .get_by_txid(txid)
        .await?
        .ok_or_else(|| DbError::QueryError(format!("Charm with txid {} not found", txid)))?;

    Ok(CharmData {
        txid: charm.txid,
        charmid: charm.charmid,
        block_height: charm.block_height,
        data: charm.data,
        date_created: charm.date_created.to_string(),
        asset_type: charm.asset_type,
    })
}
