use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::handlers::AppState;
use crate::services::asset_service::AssetService;

/// Normalize image value - handles both URLs and base64 data
/// People sometimes put URLs in the 'image' field instead of 'image_url'
fn normalize_image_value(value: &str) -> String {
    let trimmed = value.trim();

    // Already a data URI (base64), HTTP/HTTPS URL, or IPFS - return as-is
    if trimmed.starts_with("data:")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("ipfs://")
    {
        return trimmed.to_string();
    }

    // If it looks like raw base64 without prefix (long string, no spaces/slashes)
    if trimmed.len() > 100 && !trimmed.contains(' ') && !trimmed.contains('/') {
        return format!("data:image/png;base64,{}", trimmed);
    }

    // Unknown format - return as-is
    trimmed.to_string()
}

/// Extract asset metadata from charm JSONB data field
fn extract_asset_metadata_from_charm(
    charm_data: &serde_json::Value,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
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
                                        name = output_obj
                                            .get("name")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                    }
                                    if symbol.is_none() {
                                        symbol = output_obj
                                            .get("symbol")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                    }
                                    if description.is_none() {
                                        description = output_obj
                                            .get("description")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                    }
                                    if image_url.is_none() {
                                        image_url = output_obj
                                            .get("image")
                                            .or_else(|| output_obj.get("image_url"))
                                            .and_then(|v| v.as_str())
                                            .map(|s| normalize_image_value(s));
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
    pub app_id: Option<String>,
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
    pub decimals: i16, // [RJJ-DECIMALS] Dynamic decimal precision
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

/// Get assets with optional filtering by type, network, and app_id
pub async fn get_assets(
    Query(params): Query<AssetQueryParams>,
    State(state): State<AppState>,
) -> Result<Json<AssetResponse>, StatusCode> {
    let asset_service = AssetService::new(state.repositories.asset_repository.clone());

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    let offset = (page - 1) * limit;

    // If app_id is provided, search by app_id directly
    let (assets, total) = if let Some(ref app_id) = params.app_id {
        match asset_service.get_asset_by_app_id(app_id).await {
            Ok(Some(asset)) => (vec![asset], 1u64),
            Ok(None) => (vec![], 0u64),
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        match asset_service
            .get_assets_paginated(
                params.asset_type.as_deref(),
                params.network.as_deref(),
                limit,
                offset,
            )
            .await
        {
            Ok(result) => result,
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    };

    match Ok::<_, ()>((assets, total)) {
        Ok((assets, total)) => {
            // Batch fetch all charms by txids (single query instead of N+1)
            let txids: Vec<String> = assets.iter().map(|a| a.txid.clone()).collect();
            let charms = state
                .repositories
                .charm
                .get_by_txids(&txids)
                .await
                .unwrap_or_default();

            // Create lookup map for O(1) access
            let charm_map: HashMap<String, _> =
                charms.into_iter().map(|c| (c.txid.clone(), c)).collect();

            let mut asset_items = Vec::new();
            for asset in assets {
                let mut name = asset.name.clone();
                let mut symbol = asset.symbol.clone();
                let mut description = asset.description.clone();
                let mut image_url = asset.image_url.clone();

                // Use pre-fetched charm data for metadata extraction
                if let Some(charm) = charm_map.get(&asset.txid) {
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
                    total_supply: asset
                        .total_supply
                        .map(|d| d.to_string().parse::<i64>().unwrap_or(0)),
                    decimals: asset.decimals, // [RJJ-DECIMALS]
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
            tracing::error!("Error fetching assets: {:?}", e);
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
            tracing::error!("Error fetching asset counts: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Response for reference NFT metadata endpoint
#[derive(Debug, Serialize)]
pub struct ReferenceNftResponse {
    pub app_id: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub decimals: i16,
}

/// Get reference NFT metadata by hash (for token image lookup)
/// This endpoint is used by the frontend to fetch the image from the reference NFT
/// when displaying a token, avoiding storing duplicate images in the database
pub async fn get_reference_nft_by_hash(
    axum::extract::Path(hash): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> Result<Json<ReferenceNftResponse>, StatusCode> {
    let asset_service = AssetService::new(state.repositories.asset_repository.clone());

    match asset_service.get_reference_nft_by_hash(&hash).await {
        Ok(Some(nft)) => {
            let mut image_url = nft.image_url;

            // If no image_url in asset, try to extract from charm data
            if image_url.is_none() {
                if let Ok(Some(charm)) = state.repositories.charm.get_by_txid(&nft.txid).await {
                    let (_, _, _, charm_image_url) = extract_asset_metadata_from_charm(&charm.data);
                    image_url = charm_image_url;
                }
            }

            Ok(Json(ReferenceNftResponse {
                app_id: nft.app_id,
                name: nft.name,
                symbol: nft.symbol,
                description: nft.description,
                image_url,
                decimals: nft.decimals,
            }))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Error fetching reference NFT by hash: {:?}", e);
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
            let _decimals = asset.decimals; // Keep for potential future use

            // [RJJ-TOKEN-METADATA] If this is a token, try to inherit metadata from reference NFT
            let total_supply = asset.total_supply;
            if asset.app_id.starts_with("t/") {
                // Convert t/HASH/... to n/HASH/... to find reference NFT
                let nft_app_id = asset.app_id.replacen("t/", "n/", 1);

                // Try to find the reference NFT
                if let Ok(Some(nft_asset)) = asset_service.get_asset_by_app_id(&nft_app_id).await {
                    // Inherit metadata from NFT if token doesn't have it
                    name = name.or(nft_asset.name);
                    symbol = symbol.or(nft_asset.symbol);
                    description = description.or(nft_asset.description);
                    image_url = image_url.or(nft_asset.image_url);
                }
            }

            // Try to fetch related charm data for metadata extraction (as fallback)
            if name.is_none() || symbol.is_none() || description.is_none() || image_url.is_none() {
                if let Ok(Some(charm)) = state.repositories.charm.get_by_txid(&asset.txid).await {
                    let (charm_name, charm_symbol, charm_description, charm_image_url) =
                        extract_asset_metadata_from_charm(&charm.data);

                    // Use charm metadata as fallback if asset doesn't have it
                    name = name.or(charm_name);
                    symbol = symbol.or(charm_symbol);
                    description = description.or(charm_description);
                    image_url = image_url.or(charm_image_url);
                }
            }

            let asset_item = AssetItem {
                id: asset.id.to_string(),
                app_id: asset.app_id,
                asset_type: asset.asset_type,
                name,
                symbol,
                description,
                image_url,
                total_supply: total_supply.map(|d| d.to_string().parse::<i64>().unwrap_or(0)),
                decimals: asset.decimals, // [RJJ-DECIMALS]
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
            tracing::error!("Error fetching asset by ID: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
