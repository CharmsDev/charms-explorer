use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::handlers::AppState;
use crate::services::asset_service::AssetService;

/// Extract asset metadata from charm JSONB data field
fn extract_asset_metadata_from_charm(charm_data: &serde_json::Value) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let mut name = None;
    let mut symbol = None;
    let mut description = None;
    let mut image_url = None;

    // Extract from native_data structure in charm data
    if let Some(native_data) = charm_data.get("native_data") {
        if let Some(tx) = native_data.get("tx") {
            if let Some(outs) = tx.get("outs") {
                if let Some(outs_array) = outs.as_array() {
                    // Look through outputs for metadata
                    for out in outs_array {
                        if let Some(out_obj) = out.as_object() {
                            for (_, output_data) in out_obj {
                                if let Some(output_obj) = output_data.as_object() {
                                    if name.is_none() {
                                        name = output_obj.get("name")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                    }
                                    if symbol.is_none() {
                                        symbol = output_obj.get("symbol")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                    }
                                    if description.is_none() {
                                        description = output_obj.get("description")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                    }
                                    if image_url.is_none() {
                                        image_url = output_obj.get("image")
                                            .or_else(|| output_obj.get("image_url"))
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
        
    (name, symbol, description, image_url)
}

#[derive(Debug, Deserialize)]
pub struct AssetQueryParams {
    pub asset_type: Option<String>,
    pub network: Option<String>,
    pub page: Option<u64>,
    pub limit: Option<u64>,
    #[allow(dead_code)]
    pub sort: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssetResponse {
    pub data: AssetData,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize)]
pub struct AssetData {
    pub assets: Vec<AssetItem>,
}

#[derive(Debug, Serialize)]
pub struct AssetItem {
    pub id: String,
    pub app_id: String,
    pub asset_type: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub total_supply: Option<i64>,
    pub decimals: i16, // Dynamic decimal precision
    pub network: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    // Additional fields for compatibility with charm structure
    pub block_height: Option<i32>,
    pub transaction_hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub page: u64,
    pub limit: u64,
    pub total: u64,
    pub total_pages: u64,
}

/// Get assets with optional filtering by type and network
pub async fn get_assets(
    Query(params): Query<AssetQueryParams>,
    State(state): State<AppState>,
) -> Result<Json<AssetResponse>, StatusCode> {
    let asset_service = AssetService::new(state.repositories.asset_repository.clone());

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    let offset = (page - 1) * limit;

    match asset_service
        .get_assets_paginated(
            params.asset_type.as_deref(),
            params.network.as_deref(),
            limit,
            offset,
        )
        .await
    {
        Ok((assets, total)) => {
            // Fetch charm data for each asset to extract rich metadata
            let mut asset_items = Vec::new();
            for asset in assets {
                let mut name = asset.name;
                let mut symbol = asset.symbol;
                let mut description = asset.description;
                let mut image_url = asset.image_url;

                // Try to fetch related charm data for metadata extraction
                if let Ok(Some(charm)) = state.repositories.charm.get_by_txid(&asset.txid).await {
                    let (charm_name, charm_symbol, charm_description, charm_image_url) = 
                        extract_asset_metadata_from_charm(&charm.data);
                    
                    // Use charm metadata as fallback if asset doesn't have it
                    name = name.or(charm_name);
                    symbol = symbol.or(charm_symbol);
                    description = description.or(charm_description);
                    image_url = image_url.or(charm_image_url);
                }

                asset_items.push(AssetItem {
                    id: asset.id.to_string(),
                    app_id: asset.app_id,
                    asset_type: asset.asset_type,
                    name,
                    symbol,
                    description,
                    image_url,
                    total_supply: asset.total_supply.map(|d| d.to_string().parse::<i64>().unwrap_or(0)),
                    decimals: asset.decimals,
                    network: asset.network,
                    created_at: asset.created_at,
                    updated_at: asset.updated_at,
                    block_height: Some(asset.block_height),
                    transaction_hash: Some(asset.txid),
                });
            }

            let total_pages = total.div_ceil(limit); // Ceiling division

            let response = AssetResponse {
                data: AssetData {
                    assets: asset_items,
                },
                pagination: PaginationInfo {
                    page,
                    limit,
                    total,
                    total_pages,
                },
            };

            Ok(Json(response))
        }
        Err(e) => {
            eprintln!("Error fetching assets: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get asset counts by type
pub async fn get_asset_counts(
    State(state): State<AppState>,
) -> Result<Json<HashMap<String, u64>>, StatusCode> {
    let asset_service = AssetService::new(state.repositories.asset_repository.clone());

    match asset_service.get_asset_counts().await {
        Ok(counts) => Ok(Json(counts)),
        Err(e) => {
            eprintln!("Error fetching asset counts: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get a specific asset by ID
pub async fn get_asset_by_id(
    axum::extract::Path(asset_id): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> Result<Json<AssetItem>, StatusCode> {
    let asset_service = AssetService::new(state.repositories.asset_repository.clone());

    // Try to parse asset_id as UUID, if it fails try as app_id
    let asset_result = if let Ok(id) = asset_id.parse::<i32>() {
        asset_service.get_asset_by_id(id).await
    } else {
        asset_service.get_asset_by_app_id(&asset_id).await
    };

    match asset_result {
        Ok(Some(asset)) => {
            let mut name = asset.name;
            let mut symbol = asset.symbol;
            let mut description = asset.description;
            let mut image_url = asset.image_url;

            // Try to fetch related charm data for metadata extraction
            if let Ok(Some(charm)) = state.repositories.charm.get_by_txid(&asset.txid).await {
                let (charm_name, charm_symbol, charm_description, charm_image_url) = 
                    extract_asset_metadata_from_charm(&charm.data);
                
                // Use charm metadata as fallback if asset doesn't have it
                name = name.or(charm_name);
                symbol = symbol.or(charm_symbol);
                description = description.or(charm_description);
                image_url = image_url.or(charm_image_url);
            }

            let asset_item = AssetItem {
                id: asset.id.to_string(),
                app_id: asset.app_id,
                asset_type: asset.asset_type,
                name,
                symbol,
                description,
                image_url,
                total_supply: asset.total_supply.map(|d| d.to_string().parse::<i64>().unwrap_or(0)),
                decimals: asset.decimals,
                network: asset.network,
                created_at: asset.created_at,
                updated_at: asset.updated_at,
                block_height: Some(asset.block_height),
                transaction_hash: Some(asset.txid),
            };

            Ok(Json(asset_item))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            eprintln!("Error fetching asset by ID: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
