// Charm-related business logic implementation

use crate::db::DbError;
use crate::error::ExplorerResult;
use crate::handlers::AppState;
use crate::models::{CharmCountResponse, CharmData, CharmsResponse};

pub async fn get_charm_numbers_by_type(
    state: &AppState,
    asset_type: Option<&str>,
) -> ExplorerResult<CharmCountResponse> {
    let charm_numbers = state
        .repositories
        .charm
        .get_charm_numbers_by_type(asset_type)
        .await?;
    Ok(CharmCountResponse {
        count: charm_numbers.len(),
    })
}

pub async fn get_all_charms(state: &AppState) -> ExplorerResult<CharmsResponse> {
    let charms = state.repositories.charm.get_all().await?;
    let charm_data = charms
        .into_iter()
        .filter(|charm| !is_empty_spell_charm(&charm.data))
        .map(|charm| CharmData {
            txid: charm.txid,
            charmid: charm.charmid,
            block_height: charm.block_height,
            data: charm.data.clone(),
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
    let charms = state
        .repositories
        .charm
        .find_by_asset_type(asset_type)
        .await?;
    let charm_data = charms
        .into_iter()
        .filter(|charm| !is_empty_spell_charm(&charm.data))
        .map(|charm| CharmData {
            txid: charm.txid,
            charmid: charm.charmid,
            block_height: charm.block_height,
            data: charm.data.clone(),
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
        })
        .collect();
    Ok(CharmsResponse { charms: charm_data })
}

/// Checks if a charm is an empty spell charm with the structure {"data": {}, "type": "spell", "detected": true}
fn is_empty_spell_charm(data: &serde_json::Value) -> bool {
    if let Some(data_obj) = data.get("data") {
        if data_obj.is_object() && data_obj.as_object().unwrap().is_empty() {
            if let Some(type_value) = data.get("type") {
                if type_value.is_string() && type_value.as_str().unwrap() == "spell" {
                    if let Some(detected) = data.get("detected") {
                        return detected.is_boolean() && detected.as_bool().unwrap();
                    }
                }
            }
        }
    }
    false
}

pub async fn get_charm_by_txid(state: &AppState, txid: &str) -> ExplorerResult<CharmData> {
    let charm = state
        .repositories
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

/// Gets a charm by its charm ID
pub async fn get_charm_by_charmid(state: &AppState, charmid: &str) -> ExplorerResult<CharmData> {
    let charms = state.repositories.charm.find_by_charmid(charmid).await?;

    if charms.is_empty() {
        return Err(
            DbError::QueryError(format!("Charm with charmid {} not found", charmid)).into(),
        );
    }

    // First try to find a non-empty spell charm
    for charm in &charms {
        if !is_empty_spell_charm(&charm.data) {
            return Ok(CharmData {
                txid: charm.txid.clone(),
                charmid: charm.charmid.clone(),
                block_height: charm.block_height,
                data: charm.data.clone(),
                date_created: charm.date_created.to_string(),
                asset_type: charm.asset_type.clone(),
            });
        }
    }

    // If all are empty spell charms, return the first one
    let first_charm = &charms[0];
    Ok(CharmData {
        txid: first_charm.txid.clone(),
        charmid: first_charm.charmid.clone(),
        block_height: first_charm.block_height,
        data: first_charm.data.clone(),
        date_created: first_charm.date_created.to_string(),
        asset_type: first_charm.asset_type.clone(),
    })
}
