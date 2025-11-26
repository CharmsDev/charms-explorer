// Charm-related business logic implementation

use crate::db::DbError;
use crate::error::ExplorerResult;
use crate::handlers::AppState;
use crate::models::{CharmCountResponse, CharmData, CharmsResponse, LikeCharmRequest, LikeResponse, PaginatedResponse, PaginationMeta, PaginationParams};

pub async fn get_charm_numbers_by_type(
    state: &AppState,
    asset_type: Option<&str>,
) -> ExplorerResult<CharmCountResponse> {
    // Wrap the database call in a try-catch to provide more detailed error information
    let charm_numbers = match state
        .repositories
        .charm
        .get_charm_numbers_by_type(asset_type)
        .await {
            Ok(result) => result,
            Err(err) => {
                // Log the error for debugging
                eprintln!("Database error in get_charm_numbers_by_type: {:?}", err);
                
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
    let (charms, total) = match state.repositories.charm.get_all_paginated_by_network(pagination, network_str).await {
        Ok(result) => result,
        Err(err) => {
            // Log database error for monitoring
            eprintln!("Database error in get_all_charms_paginated_by_network: {:?}", err);
            
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
    
    let mut charm_data = Vec::new();
    
    for charm in charms {
        if is_empty_spell_charm(&charm.data) {
            continue;
        }
        
        let charm_data_item = CharmData {
            txid: charm.txid,
            charmid: charm.app_id.clone(),
            block_height: charm.block_height,
            data: charm.data,
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
            likes_count: 0,
            user_liked: false,
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
            eprintln!("Database error in get_all_charms_paginated: {:?}", err);
            
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
    
    let mut charm_data = Vec::new();
    
    for charm in charms {
        if is_empty_spell_charm(&charm.data) {
            continue;
        }
        
        // Get likes count for this charm
        let likes_count = (state.repositories.likes.get_likes_count(&charm.app_id).await).unwrap_or(0);
        
        // Check if the user has liked this charm
        let user_liked = (state.repositories.likes.has_user_liked(&charm.app_id, user_id).await).unwrap_or(false);
        
        charm_data.push(CharmData {
            txid: charm.txid,
            charmid: charm.app_id.clone(),
            block_height: charm.block_height,
            data: charm.data.clone(),
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
            likes_count,
            user_liked,
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
            eprintln!("Database error in get_all_charms: {:?}", err);
            
            // Return an empty vector instead of propagating the error
            vec![]
        }
    };
    
    let mut charm_data = Vec::new();
    
    for charm in charms {
        if is_empty_spell_charm(&charm.data) {
            continue;
        }
        
        // Get likes count for this charm
        let likes_count = (state.repositories.likes.get_likes_count(&charm.app_id).await).unwrap_or(0);
        
        // Check if the user has liked this charm
        let user_liked = (state.repositories.likes.has_user_liked(&charm.app_id, user_id).await).unwrap_or(false);
        
        charm_data.push(CharmData {
            txid: charm.txid,
            charmid: charm.app_id.clone(),
            block_height: charm.block_height,
            data: charm.data.clone(),
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
            likes_count,
            user_liked,
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
        .await {
            Ok(result) => result,
            Err(err) => {
                // Log the error for debugging
                eprintln!("Database error in get_charms_by_type_paginated: {:?}", err);
                
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
    
    let mut charm_data = Vec::new();
    
    for charm in charms {
        if is_empty_spell_charm(&charm.data) {
            continue;
        }
        
        // Get likes count for this charm
        let likes_count = (state.repositories.likes.get_likes_count(&charm.app_id).await).unwrap_or(0);
        
        // Check if the user has liked this charm
        let user_liked = (state.repositories.likes.has_user_liked(&charm.app_id, user_id).await).unwrap_or(false);
        
        charm_data.push(CharmData {
            txid: charm.txid,
            charmid: charm.app_id.clone(),
            block_height: charm.block_height,
            data: charm.data.clone(),
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
            likes_count,
            user_liked,
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
        .await {
            Ok(result) => result,
            Err(err) => {
                // Log the error for debugging
                eprintln!("Database error in get_charms_by_type: {:?}", err);
                
                // Return an empty vector instead of propagating the error
                vec![]
            }
        };
    
    let mut charm_data = Vec::new();
    
    for charm in charms {
        if is_empty_spell_charm(&charm.data) {
            continue;
        }
        
        // Get likes count for this charm
        let likes_count = (state.repositories.likes.get_likes_count(&charm.app_id).await).unwrap_or(0);
        
        // Check if the user has liked this charm
        let user_liked = (state.repositories.likes.has_user_liked(&charm.app_id, user_id).await).unwrap_or(false);
        
        charm_data.push(CharmData {
            txid: charm.txid,
            charmid: charm.app_id.clone(),
            block_height: charm.block_height,
            data: charm.data.clone(),
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
            likes_count,
            user_liked,
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

pub async fn get_charm_by_txid(state: &AppState, txid: &str, user_id: i32) -> ExplorerResult<CharmData> {
    // Wrap the database call in a try-catch to provide more detailed error information
    let charm_result = match state.repositories.charm.get_by_txid(txid).await {
        Ok(result) => result,
        Err(err) => {
            // Log the error for debugging
            eprintln!("Database error in get_charm_by_txid: {:?}", err);
            
            // Return a not found error with a friendly message
            return Err(DbError::QueryError(format!("Charm with txid {} not found or database error occurred", txid)).into());
        }
    };
    
    // Check if the charm was found
    let charm = match charm_result {
        Some(charm) => charm,
        None => {
            return Err(DbError::QueryError(format!("Charm with txid {} not found", txid)).into());
        }
    };

    // Get likes count for this charm
    let likes_count = (state.repositories.likes.get_likes_count(&charm.app_id).await).unwrap_or(0);
    
    // Check if the user has liked this charm
    let user_liked = (state.repositories.likes.has_user_liked(&charm.app_id, user_id).await).unwrap_or(false);

    Ok(CharmData {
        txid: charm.txid,
        charmid: charm.app_id,
        block_height: charm.block_height,
        data: charm.data,
        date_created: charm.date_created.to_string(),
        asset_type: charm.asset_type,
        likes_count,
        user_liked,
    })
}

/// Gets a charm by its charm ID
pub async fn get_charm_by_charmid(state: &AppState, charmid: &str, user_id: i32) -> ExplorerResult<CharmData> {
    // Wrap the database call in a try-catch to provide more detailed error information
    let charms = match state.repositories.charm.find_by_charmid(charmid).await {
        Ok(result) => result,
        Err(err) => {
            // Log the error for debugging
            eprintln!("Database error in get_charm_by_charmid: {:?}", err);
            
            // Return a not found error with a friendly message
            return Err(DbError::QueryError(format!("Charm with charmid {} not found or database error occurred", charmid)).into());
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
    let user_liked = (state.repositories.likes.has_user_liked(charmid, user_id).await).unwrap_or(false);

    // First try to find a non-empty spell charm
    for charm in &charms {
        if !is_empty_spell_charm(&charm.data) {
            return Ok(CharmData {
                txid: charm.txid.clone(),
                charmid: charm.app_id.clone(),
                block_height: charm.block_height,
                data: charm.data.clone(),
                date_created: charm.date_created.to_string(),
                asset_type: charm.asset_type.clone(),
                likes_count,
                user_liked,
            });
        }
    }

    // If all are empty spell charms, return the first one
    let first_charm = &charms[0];
    Ok(CharmData {
        txid: first_charm.txid.clone(),
        charmid: first_charm.app_id.clone(),
        block_height: first_charm.block_height,
        data: first_charm.data.clone(),
        date_created: first_charm.date_created.to_string(),
        asset_type: first_charm.asset_type.clone(),
        likes_count,
        user_liked,
    })
}

/// Adds a like to a charm
pub async fn add_like(state: &AppState, request: &LikeCharmRequest) -> ExplorerResult<LikeResponse> {
    // Add the like
    match state.repositories.likes.add_like(&request.charm_id, request.user_id).await {
        Ok(likes_count) => {
            
            Ok(LikeResponse {
                success: true,
                message: "Like added successfully".to_string(),
                likes_count,
            })
        },
        Err(err) => {
            eprintln!("Database error in add_like: {:?}", err);
            
            Err(DbError::QueryError("Failed to add like".to_string()).into())
        }
    }
}

/// Removes a like from a charm
pub async fn remove_like(state: &AppState, request: &LikeCharmRequest) -> ExplorerResult<LikeResponse> {
    // Remove the like
    match state.repositories.likes.remove_like(&request.charm_id, request.user_id).await {
        Ok(likes_count) => {
            
            Ok(LikeResponse {
                success: true,
                message: "Like removed successfully".to_string(),
                likes_count,
            })
        },
        Err(err) => {
            eprintln!("Database error in remove_like: {:?}", err);
            
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
            eprintln!("Database error in get_charms_by_address: {:?}", err);
            return Ok(CharmsResponse { charms: vec![] });
        }
    };

    // Transform charms to CharmData format
    let mut charm_data = Vec::new();
    
    for charm in charms {
        // Get likes count for this charm
        let likes_count = state.repositories.likes.get_likes_count(&charm.app_id).await.unwrap_or(0);
        
        // Check if the user has liked this charm
        let user_liked = state.repositories.likes.has_user_liked(&charm.app_id, user_id).await.unwrap_or(false);

        charm_data.push(CharmData {
            txid: charm.txid,
            charmid: charm.app_id.clone(), // Using app_id as charmid
            block_height: charm.block_height,
            data: charm.data,
            date_created: charm.date_created.to_string(),
            asset_type: charm.asset_type,
            likes_count,
            user_liked,
        });
    }

    Ok(CharmsResponse { charms: charm_data })
}

/// Extract hash from app_id (removes t/ or n/ prefix) [RJJ-ADDRESS-SEARCH]
fn extract_hash_from_app_id(app_id: &str) -> String {
    let parts: Vec<&str> = app_id.split('/').collect();
    if parts.len() >= 2 {
        parts[1..].join("/")
    } else {
        app_id.to_string()
    }
}
