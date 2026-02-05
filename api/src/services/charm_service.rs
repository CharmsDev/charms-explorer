// Charm-related business logic implementation

use std::collections::{HashMap, HashSet};

use crate::db::DbError;
use crate::error::ExplorerResult;
use crate::handlers::AppState;
use crate::models::{
    CharmCountResponse, CharmData, CharmsCountByTypeResponse, CharmsResponse, LikeCharmRequest,
    LikeResponse, PaginatedResponse, PaginationMeta, PaginationParams,
};

pub async fn get_charms_count_by_type(
    state: &AppState,
    network: Option<&str>,
) -> ExplorerResult<CharmsCountByTypeResponse> {
    use crate::entity::assets::{Column as AssetColumn, Entity as Assets};
    use crate::entity::charms::{Column as CharmColumn, Entity as Charms};
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    let network_str = network.unwrap_or("mainnet");
    let conn = state.repositories.charm.get_connection();

    // Count total charms
    let total = Charms::find()
        .filter(CharmColumn::Network.eq(network_str))
        .count(conn)
        .await
        .unwrap_or(0);

    // Count assets by type (unique assets, not charm instances)
    let nft_count = Assets::find()
        .filter(AssetColumn::Network.eq(network_str))
        .filter(AssetColumn::AssetType.eq("nft"))
        .count(conn)
        .await
        .unwrap_or(0);

    let token_count = Assets::find()
        .filter(AssetColumn::Network.eq(network_str))
        .filter(AssetColumn::AssetType.eq("token"))
        .count(conn)
        .await
        .unwrap_or(0);

    let dapp_count = Assets::find()
        .filter(AssetColumn::Network.eq(network_str))
        .filter(AssetColumn::AssetType.eq("dapp"))
        .count(conn)
        .await
        .unwrap_or(0);

    Ok(CharmsCountByTypeResponse {
        total,
        nft: nft_count,
        token: token_count,
        dapp: dapp_count,
    })
}

pub async fn get_charm_numbers_by_type(
    state: &AppState,
    asset_type: Option<&str>,
) -> ExplorerResult<CharmCountResponse> {
    // Wrap the database call in a try-catch to provide more detailed error information
    let charm_numbers = match state
        .repositories
        .charm
        .get_charm_numbers_by_type(asset_type)
        .await
    {
        Ok(result) => result,
        Err(err) => {
            // Log the error for debugging
            tracing::warn!("Database error in get_charm_numbers_by_type: {:?}", err);

            // Return an empty vector instead of propagating the error
            vec![]
        }
    };

    Ok(CharmCountResponse {
        count: charm_numbers.len(),
    })
}

pub async fn get_all_charms_paginated_by_network(
    state: &AppState,
    pagination: &PaginationParams,
    _user_id: i32,
    network: Option<&str>,
) -> ExplorerResult<PaginatedResponse<CharmsResponse>> {
    // Handle database query with graceful error handling
    let network_str = network.unwrap_or("mainnet");
    let (charms, total) = match state
        .repositories
        .charm
        .get_all_paginated_by_network(pagination, network_str)
        .await
    {
        Ok(result) => result,
        Err(err) => {
            // Log database error for monitoring
            tracing::warn!(
                "Database error in get_all_charms_paginated_by_network: {:?}",
                err
            );

            // Return empty response on database error
            return Ok(PaginatedResponse {
                data: CharmsResponse { charms: vec![] },
                pagination: PaginationMeta {
                    total: 0,
                    page: pagination.page,
                    limit: pagination.limit,
                    total_pages: 0,
                },
            });
        }
    };

    // [RJJ-METADATA] Enrich charms with metadata from assets table
    let app_ids: Vec<String> = charms.iter().map(|c| c.app_id.clone()).collect();
    let metadata_map = get_metadata_map(state, app_ids).await;

    let mut charm_data = Vec::new();

    for charm in charms {
        if is_empty_spell_charm(&charm.data) {
            continue;
        }

        // Get metadata for this charm
        let (name, image, ticker, description) = metadata_map
            .get(&charm.app_id)
            .cloned()
            .unwrap_or((None, None, None, None));

        let charm_data_item = CharmData {
            txid: charm.txid,
            vout: charm.vout,
            charmid: charm.app_id.clone(),
            block_height: charm.block_height,
            data: charm.data,
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
            network: charm.network,
            amount: charm.amount,
            likes_count: 0,
            user_liked: false,
            name,
            image,
            ticker,
            description,
            verified: charm.verified,
            spell: None,
        };

        charm_data.push(charm_data_item);
    }

    let total_pages = if pagination.limit > 0 {
        (total as f64 / pagination.limit as f64).ceil() as u64
    } else {
        1
    };

    Ok(PaginatedResponse {
        data: CharmsResponse { charms: charm_data },
        pagination: PaginationMeta {
            total,
            page: pagination.page,
            limit: pagination.limit,
            total_pages,
        },
    })
}

pub async fn get_all_charms_paginated(
    state: &AppState,
    pagination: &PaginationParams,
    user_id: i32,
) -> ExplorerResult<PaginatedResponse<CharmsResponse>> {
    // Handle database query with graceful error handling
    let (charms, total) = match state.repositories.charm.get_all_paginated(pagination).await {
        Ok(result) => result,
        Err(err) => {
            // Log database error for monitoring
            tracing::warn!("Database error in get_all_charms_paginated: {:?}", err);

            // Return empty response on database error
            return Ok(PaginatedResponse {
                data: CharmsResponse { charms: vec![] },
                pagination: PaginationMeta {
                    total: 0,
                    page: pagination.page,
                    limit: pagination.limit,
                    total_pages: 0,
                },
            });
        }
    };

    // [RJJ-METADATA] Enrich charms with metadata from assets table
    let app_ids: Vec<String> = charms.iter().map(|c| c.app_id.clone()).collect();
    let metadata_map = get_metadata_map(state, app_ids.clone()).await;

    // [RJJ-PERF] Batch fetch likes data (2 queries instead of 2N)
    let likes_counts = state
        .repositories
        .likes
        .get_likes_counts_batch(&app_ids)
        .await
        .unwrap_or_default();
    let user_likes = state
        .repositories
        .likes
        .get_user_likes_batch(&app_ids, user_id)
        .await
        .unwrap_or_default();

    let mut charm_data = Vec::new();

    for charm in charms {
        if is_empty_spell_charm(&charm.data) {
            continue;
        }

        // Get likes from batch results
        let likes_count = *likes_counts.get(&charm.app_id).unwrap_or(&0);
        let user_liked = user_likes.contains(&charm.app_id);

        // Get metadata for this charm
        let (name, image, ticker, description) = metadata_map
            .get(&charm.app_id)
            .cloned()
            .unwrap_or((None, None, None, None));

        charm_data.push(CharmData {
            txid: charm.txid,
            vout: charm.vout,
            charmid: charm.app_id.clone(),
            block_height: charm.block_height,
            data: charm.data.clone(),
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
            network: charm.network,
            amount: charm.amount,
            likes_count,
            user_liked,
            name,
            image,
            ticker,
            description,
            verified: charm.verified,
            spell: None,
        });
    }

    let total_pages = if pagination.limit > 0 {
        (total as f64 / pagination.limit as f64).ceil() as u64
    } else {
        1 // Avoid division by zero
    };

    Ok(PaginatedResponse {
        data: CharmsResponse { charms: charm_data },
        pagination: PaginationMeta {
            total,
            page: pagination.page,
            limit: pagination.limit,
            total_pages,
        },
    })
}

#[allow(dead_code)]
pub async fn get_all_charms(state: &AppState, user_id: i32) -> ExplorerResult<CharmsResponse> {
    // Wrap the database call in a try-catch to provide more detailed error information
    let charms = match state.repositories.charm.get_all().await {
        Ok(result) => result,
        Err(err) => {
            // Log the error for debugging
            tracing::warn!("Database error in get_all_charms: {:?}", err);

            // Return an empty vector instead of propagating the error
            vec![]
        }
    };

    // [RJJ-METADATA] Enrich charms with metadata from assets table
    let app_ids: Vec<String> = charms.iter().map(|c| c.app_id.clone()).collect();
    let metadata_map = get_metadata_map(state, app_ids).await;

    let mut charm_data = Vec::new();

    for charm in charms {
        if is_empty_spell_charm(&charm.data) {
            continue;
        }

        // Get likes count for this charm
        let likes_count = (state
            .repositories
            .likes
            .get_likes_count(&charm.app_id)
            .await)
            .unwrap_or(0);

        // Check if the user has liked this charm
        let user_liked = (state
            .repositories
            .likes
            .has_user_liked(&charm.app_id, user_id)
            .await)
            .unwrap_or(false);

        // Get metadata for this charm
        let (name, image, ticker, description) = metadata_map
            .get(&charm.app_id)
            .cloned()
            .unwrap_or((None, None, None, None));

        charm_data.push(CharmData {
            txid: charm.txid,
            vout: charm.vout,
            charmid: charm.app_id.clone(),
            block_height: charm.block_height,
            data: charm.data.clone(),
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
            network: charm.network,
            amount: charm.amount,
            likes_count,
            user_liked,
            name,
            image,
            ticker,
            description,
            verified: charm.verified,
            spell: None,
        });
    }

    Ok(CharmsResponse { charms: charm_data })
}

pub async fn get_charms_by_type_paginated(
    state: &AppState,
    asset_type: &str,
    pagination: &PaginationParams,
    user_id: i32,
) -> ExplorerResult<PaginatedResponse<CharmsResponse>> {
    // Wrap the database call in a try-catch to provide more detailed error information
    let (charms, total) = match state
        .repositories
        .charm
        .find_by_asset_type_paginated(asset_type, pagination)
        .await
    {
        Ok(result) => result,
        Err(err) => {
            // Log the error for debugging
            tracing::warn!("Database error in get_charms_by_type_paginated: {:?}", err);

            // Return a fallback empty response instead of propagating the error
            return Ok(PaginatedResponse {
                data: CharmsResponse { charms: vec![] },
                pagination: PaginationMeta {
                    total: 0,
                    page: pagination.page,
                    limit: pagination.limit,
                    total_pages: 0,
                },
            });
        }
    };

    // [RJJ-METADATA] Enrich charms with metadata from assets table
    let app_ids: Vec<String> = charms.iter().map(|c| c.app_id.clone()).collect();
    let metadata_map = get_metadata_map(state, app_ids.clone()).await;

    // [RJJ-PERF] Batch fetch likes data (2 queries instead of 2N)
    let likes_counts = state
        .repositories
        .likes
        .get_likes_counts_batch(&app_ids)
        .await
        .unwrap_or_default();
    let user_likes = state
        .repositories
        .likes
        .get_user_likes_batch(&app_ids, user_id)
        .await
        .unwrap_or_default();

    let mut charm_data = Vec::new();

    for charm in charms {
        if is_empty_spell_charm(&charm.data) {
            continue;
        }

        // Get likes from batch results
        let likes_count = *likes_counts.get(&charm.app_id).unwrap_or(&0);
        let user_liked = user_likes.contains(&charm.app_id);

        // Get metadata for this charm
        let (name, image, ticker, description) = metadata_map
            .get(&charm.app_id)
            .cloned()
            .unwrap_or((None, None, None, None));

        charm_data.push(CharmData {
            txid: charm.txid,
            vout: charm.vout,
            charmid: charm.app_id.clone(),
            block_height: charm.block_height,
            data: charm.data.clone(),
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
            network: charm.network,
            amount: charm.amount,
            likes_count,
            user_liked,
            name,
            image,
            ticker,
            description,
            verified: charm.verified,
            spell: None,
        });
    }

    let total_pages = if pagination.limit > 0 {
        (total as f64 / pagination.limit as f64).ceil() as u64
    } else {
        1 // Avoid division by zero
    };

    Ok(PaginatedResponse {
        data: CharmsResponse { charms: charm_data },
        pagination: PaginationMeta {
            total,
            page: pagination.page,
            limit: pagination.limit,
            total_pages,
        },
    })
}

#[allow(dead_code)]
pub async fn get_charms_by_type(
    state: &AppState,
    asset_type: &str,
    user_id: i32,
) -> ExplorerResult<CharmsResponse> {
    // Wrap the database call in a try-catch to provide more detailed error information
    let charms = match state
        .repositories
        .charm
        .find_by_asset_type(asset_type)
        .await
    {
        Ok(result) => result,
        Err(err) => {
            // Log the error for debugging
            tracing::warn!("Database error in get_charms_by_type: {:?}", err);

            // Return an empty vector instead of propagating the error
            vec![]
        }
    };

    // [RJJ-METADATA] Enrich charms with metadata from assets table
    let app_ids: Vec<String> = charms.iter().map(|c| c.app_id.clone()).collect();
    let metadata_map = get_metadata_map(state, app_ids).await;

    let mut charm_data = Vec::new();

    for charm in charms {
        if is_empty_spell_charm(&charm.data) {
            continue;
        }

        // Get likes count for this charm
        let likes_count = (state
            .repositories
            .likes
            .get_likes_count(&charm.app_id)
            .await)
            .unwrap_or(0);

        // Check if the user has liked this charm
        let user_liked = (state
            .repositories
            .likes
            .has_user_liked(&charm.app_id, user_id)
            .await)
            .unwrap_or(false);

        // Get metadata for this charm
        let (name, image, ticker, description) = metadata_map
            .get(&charm.app_id)
            .cloned()
            .unwrap_or((None, None, None, None));

        charm_data.push(CharmData {
            txid: charm.txid,
            vout: charm.vout,
            charmid: charm.app_id.clone(),
            block_height: charm.block_height,
            data: charm.data.clone(),
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
            network: charm.network,
            amount: charm.amount,
            likes_count,
            user_liked,
            name,
            image,
            ticker,
            description,
            verified: charm.verified,
            spell: None,
        });
    }

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

pub async fn get_charm_by_txid(
    state: &AppState,
    txid: &str,
    user_id: i32,
) -> ExplorerResult<CharmData> {
    // Wrap the database call in a try-catch to provide more detailed error information
    let charm_result = match state.repositories.charm.get_by_txid(txid).await {
        Ok(result) => result,
        Err(err) => {
            // Log the error for debugging
            tracing::warn!("Database error in get_charm_by_txid: {:?}", err);

            // Return a not found error with a friendly message
            return Err(DbError::QueryError(format!(
                "Charm with txid {} not found or database error occurred",
                txid
            ))
            .into());
        }
    };

    // Check if the charm was found
    let charm = match charm_result {
        Some(charm) => charm,
        None => {
            return Err(DbError::QueryError(format!("Charm with txid {} not found", txid)).into());
        }
    };

    // [RJJ-SPELL] Get original spell from transactions table
    let spell = match state
        .repositories
        .transactions
        .get_spell_by_txid(txid)
        .await
    {
        Ok(spell_opt) => spell_opt,
        Err(err) => {
            tracing::warn!("Error getting spell from transactions: {:?}", err);
            None
        }
    };

    // Get likes count for this charm
    let likes_count = (state
        .repositories
        .likes
        .get_likes_count(&charm.app_id)
        .await)
        .unwrap_or(0);

    // Check if the user has liked this charm
    let user_liked = (state
        .repositories
        .likes
        .has_user_liked(&charm.app_id, user_id)
        .await)
        .unwrap_or(false);

    // Get metadata for this charm
    let app_ids = vec![charm.app_id.clone()];
    let metadata_map = get_metadata_map(state, app_ids).await;
    let (name, image, ticker, description) = metadata_map
        .get(&charm.app_id)
        .cloned()
        .unwrap_or((None, None, None, None));

    Ok(CharmData {
        txid: charm.txid,
        vout: charm.vout,
        charmid: charm.app_id,
        block_height: charm.block_height,
        data: charm.data,
        date_created: charm.date_created.to_string(),
        asset_type: charm.asset_type,
        network: charm.network,
        amount: charm.amount,
        likes_count,
        user_liked,
        name,
        image,
        ticker,
        description,
        verified: charm.verified,
        spell, // [RJJ-SPELL] Include original spell from transactions
    })
}

/// Gets a charm by its charm ID
pub async fn get_charm_by_charmid(
    state: &AppState,
    charmid: &str,
    user_id: i32,
) -> ExplorerResult<CharmData> {
    // Wrap the database call in a try-catch to provide more detailed error information
    let charms = match state.repositories.charm.find_by_charmid(charmid).await {
        Ok(result) => result,
        Err(err) => {
            // Log the error for debugging
            tracing::warn!("Database error in get_charm_by_charmid: {:?}", err);

            // Return a not found error with a friendly message
            return Err(DbError::QueryError(format!(
                "Charm with charmid {} not found or database error occurred",
                charmid
            ))
            .into());
        }
    };

    if charms.is_empty() {
        return Err(
            DbError::QueryError(format!("Charm with charmid {} not found", charmid)).into(),
        );
    }

    // Get likes count for this charm
    let likes_count = (state.repositories.likes.get_likes_count(charmid).await).unwrap_or(0);

    // Check if the user has liked this charm
    let user_liked = (state
        .repositories
        .likes
        .has_user_liked(charmid, user_id)
        .await)
        .unwrap_or(false);

    // Get metadata for charms (all share same app_id)
    let app_ids = vec![charmid.to_string()];
    let metadata_map = get_metadata_map(state, app_ids).await;
    let (name, image, ticker, description) = metadata_map
        .get(charmid)
        .cloned()
        .unwrap_or((None, None, None, None));

    // First try to find a non-empty spell charm
    for charm in &charms {
        if !is_empty_spell_charm(&charm.data) {
            return Ok(CharmData {
                txid: charm.txid.clone(),
                vout: charm.vout,
                charmid: charm.app_id.clone(),
                block_height: charm.block_height,
                data: charm.data.clone(),
                date_created: charm.date_created.to_string(),
                asset_type: charm.asset_type.clone(),
                network: charm.network.clone(),
                amount: charm.amount,
                likes_count,
                user_liked,
                name: name.clone(),
                image: image.clone(),
                ticker: ticker.clone(),
                description: description.clone(),
                verified: charm.verified,
                spell: None,
            });
        }
    }

    // If all are empty spell charms, return the first one
    let first_charm = &charms[0];
    Ok(CharmData {
        txid: first_charm.txid.clone(),
        vout: first_charm.vout,
        charmid: first_charm.app_id.clone(),
        block_height: first_charm.block_height,
        data: first_charm.data.clone(),
        date_created: first_charm.date_created.to_string(),
        asset_type: first_charm.asset_type.clone(),
        network: first_charm.network.clone(),
        amount: first_charm.amount,
        likes_count,
        user_liked,
        name,
        image,
        ticker,
        description,
        verified: first_charm.verified,
        spell: None,
    })
}

/// Adds a like to a charm
pub async fn add_like(
    state: &AppState,
    request: &LikeCharmRequest,
) -> ExplorerResult<LikeResponse> {
    // Add the like
    match state
        .repositories
        .likes
        .add_like(&request.charm_id, request.user_id)
        .await
    {
        Ok(likes_count) => Ok(LikeResponse {
            success: true,
            message: "Like added successfully".to_string(),
            likes_count,
        }),
        Err(err) => {
            tracing::warn!("Database error in add_like: {:?}", err);

            Err(DbError::QueryError("Failed to add like".to_string()).into())
        }
    }
}

/// Removes a like from a charm
pub async fn remove_like(
    state: &AppState,
    request: &LikeCharmRequest,
) -> ExplorerResult<LikeResponse> {
    // Remove the like
    match state
        .repositories
        .likes
        .remove_like(&request.charm_id, request.user_id)
        .await
    {
        Ok(likes_count) => Ok(LikeResponse {
            success: true,
            message: "Like removed successfully".to_string(),
            likes_count,
        }),
        Err(err) => {
            tracing::warn!("Database error in remove_like: {:?}", err);

            Err(DbError::QueryError("Failed to remove like".to_string()).into())
        }
    }
}

/// [RJJ-ADDRESS-SEARCH] Get charms by address (UNSPENT only)
/// Returns charms with enriched metadata from related assets
pub async fn get_charms_by_address(
    state: &AppState,
    address: &str,
    user_id: i32,
) -> ExplorerResult<CharmsResponse> {
    // Get unspent charms for this address
    let charms = match state.repositories.charm.find_by_address(address).await {
        Ok(result) => result,
        Err(err) => {
            tracing::warn!("Database error in get_charms_by_address: {:?}", err);
            return Ok(CharmsResponse { charms: vec![] });
        }
    };

    // [RJJ-METADATA] Enrich charms with metadata from assets table
    let app_ids: Vec<String> = charms.iter().map(|c| c.app_id.clone()).collect();
    let metadata_map = get_metadata_map(state, app_ids.clone()).await;

    // [RJJ-PERF] Batch fetch likes data (2 queries instead of 2N)
    let likes_counts = state
        .repositories
        .likes
        .get_likes_counts_batch(&app_ids)
        .await
        .unwrap_or_default();
    let user_likes = state
        .repositories
        .likes
        .get_user_likes_batch(&app_ids, user_id)
        .await
        .unwrap_or_default();

    // Transform charms to CharmData format
    let mut charm_data = Vec::new();

    for charm in charms {
        // Get likes from batch results
        let likes_count = *likes_counts.get(&charm.app_id).unwrap_or(&0);
        let user_liked = user_likes.contains(&charm.app_id);

        // Get metadata for this charm
        let (name, image, ticker, description) = metadata_map
            .get(&charm.app_id)
            .cloned()
            .unwrap_or((None, None, None, None));

        charm_data.push(CharmData {
            txid: charm.txid,
            vout: charm.vout,
            charmid: charm.app_id.clone(), // Using app_id as charmid
            block_height: charm.block_height,
            data: charm.data,
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
            network: charm.network,
            amount: charm.amount,
            likes_count,
            user_liked,
            name,
            image,
            ticker,
            description,
            verified: charm.verified,
            spell: None,
        });
    }

    Ok(CharmsResponse { charms: charm_data })
}

/// Extract hash from app_id (removes t/ or n/ prefix) [RJJ-ADDRESS-SEARCH]
#[allow(dead_code)] // Reserved for future address search enhancements
fn extract_hash_from_app_id(app_id: &str) -> String {
    let parts: Vec<&str> = app_id.split('/').collect();
    if parts.len() >= 2 {
        parts[1..].join("/")
    } else {
        app_id.to_string()
    }
}

// Helper to enrich charms with metadata from assets table
async fn get_metadata_map(
    state: &AppState,
    app_ids: Vec<String>,
) -> HashMap<
    String,
    (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ),
> {
    let unique_app_ids: Vec<String> = app_ids
        .iter()
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if unique_app_ids.is_empty() {
        return HashMap::new();
    }

    let assets = match state
        .repositories
        .asset_repository
        .find_by_app_ids(unique_app_ids)
        .await
    {
        Ok(assets) => assets,
        Err(err) => {
            tracing::warn!("Error fetching assets metadata: {:?}", err);
            vec![]
        }
    };

    let mut map = HashMap::new();
    for asset in assets {
        map.insert(
            asset.app_id,
            (asset.name, asset.image_url, asset.symbol, asset.description),
        );
    }
    map
}
