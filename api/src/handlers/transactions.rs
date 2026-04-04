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

    // Parse spell's app_public_inputs to get ALL involved app_ids (including consumed inputs)
    let spell_app_ids: Vec<String> = data
        .charm
        .pointer("/native_data/app_public_inputs")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    // Parse spell outs to know which app indices have outputs
    let spell_outs: Vec<serde_json::Value> = data
        .charm
        .pointer("/native_data/tx/outs")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Parse beamed_outs to know which outputs are burned (beam-out to Cardano)
    let beamed_out_indices: std::collections::HashSet<String> = data
        .charm
        .pointer("/native_data/tx/beamed_outs")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    // Enrich with asset metadata if this transaction has charms
    if let Ok(charms) = state.repositories.charm.get_by_txids(&[txid]).await {
        // Collect all app_ids: from charms + from spell app_public_inputs
        let mut all_app_ids: Vec<String> = charms.iter().map(|c| c.app_id.clone()).collect();
        for app_id in &spell_app_ids {
            if !all_app_ids.contains(app_id) {
                all_app_ids.push(app_id.clone());
            }
        }

        // Lookup asset metadata for ALL involved app_ids
        let assets_meta = state
            .repositories
            .asset_repository
            .find_by_app_ids(all_app_ids)
            .await
            .unwrap_or_default();

        let meta_map: std::collections::HashMap<String, &crate::entity::assets::Model> =
            assets_meta.iter().map(|a| (a.app_id.clone(), a)).collect();

        // Build output assets from charms table
        let charm_app_ids: std::collections::HashSet<String> =
            charms.iter().map(|c| c.app_id.clone()).collect();

        // Build lookup for contract name inheritance: c/{hash} → t/{hash} metadata
        let contract_token_map: std::collections::HashMap<String, &crate::entity::assets::Model> =
            meta_map
                .iter()
                .filter(|(k, _)| k.starts_with("t/"))
                .map(|(k, v)| {
                    let contract_id = format!("c/{}", &k[2..]);
                    (contract_id, *v)
                })
                .collect();

        let mut assets: Vec<TransactionAsset> = charms
            .iter()
            .map(|charm| {
                let meta = meta_map.get(&charm.app_id);
                // For contracts, inherit name from matching token
                let contract_meta = if charm.app_id.starts_with("c/") {
                    contract_token_map.get(&charm.app_id)
                } else {
                    None
                };
                let role = if charm.app_id.starts_with("c/") {
                    "contract".to_string()
                } else if beamed_out_indices.contains(&charm.vout.to_string()) {
                    "beamed".to_string()
                } else {
                    "output".to_string()
                };
                // Contract inherits name from matching token (e.g. "eBTC Bridge")
                let effective_name = meta.and_then(|m| m.name.clone())
                    .or_else(|| contract_meta.and_then(|m| m.name.as_ref().map(|n| format!("{} Bridge", n))));
                let effective_symbol = meta.and_then(|m| m.symbol.clone())
                    .or_else(|| contract_meta.and_then(|m| m.symbol.clone()));
                TransactionAsset {
                    app_id: charm.app_id.clone(),
                    name: effective_name,
                    symbol: effective_symbol,
                    description: meta.and_then(|m| m.description.clone()),
                    image_url: meta.and_then(|m| m.image_url.clone())
                        .or_else(|| contract_meta.and_then(|m| m.image_url.clone())),
                    amount: charm.amount,
                    asset_type: if contract_meta.is_some() && charm.asset_type == "unknown" { "contract".to_string() } else { charm.asset_type.clone() },
                    role,
                    vout: charm.vout,
                    address: charm.address.clone(),
                    verified: charm.verified,
                    cardano_policy_id: meta.and_then(|m| m.cardano_policy_id.clone()),
                    cardano_asset_name: meta.and_then(|m| m.cardano_asset_name.clone()),
                    cardano_fingerprint: meta.and_then(|m| m.cardano_fingerprint.clone()),
                }
            })
            .collect();

        // Add consumed input tokens from spell that aren't in charms table
        // These are tokens in app_public_inputs that have no output in spell outs
        for (app_idx, app_id) in spell_app_ids.iter().enumerate() {
            if charm_app_ids.contains(app_id) || app_id.starts_with("c/") {
                continue; // Already in charms or is a contract
            }
            // Check if this app index appears in any output
            let has_output = spell_outs.iter().any(|out| {
                out.as_object()
                    .map(|obj| obj.contains_key(&app_idx.to_string()))
                    .unwrap_or(false)
            });
            if !has_output {
                // This is a consumed input token
                let meta = meta_map.get(app_id);
                let asset_type = if app_id.starts_with("t/") {
                    "token"
                } else if app_id.starts_with("n/") {
                    "nft"
                } else {
                    "unknown"
                };
                assets.push(TransactionAsset {
                    app_id: app_id.clone(),
                    name: meta.and_then(|m| m.name.clone()),
                    symbol: meta.and_then(|m| m.symbol.clone()),
                    description: meta.and_then(|m| m.description.clone()),
                    image_url: meta.and_then(|m| m.image_url.clone()),
                    amount: 0,
                    asset_type: asset_type.to_string(),
                    role: "input".to_string(),
                    vout: -1,
                    address: None,
                    verified: meta.is_some(),
                    cardano_policy_id: meta.and_then(|m| m.cardano_policy_id.clone()),
                    cardano_asset_name: meta.and_then(|m| m.cardano_asset_name.clone()),
                    cardano_fingerprint: meta.and_then(|m| m.cardano_fingerprint.clone()),
                });
            }
        }

        data.assets = assets;
    }

    Ok(Json(data))
}
