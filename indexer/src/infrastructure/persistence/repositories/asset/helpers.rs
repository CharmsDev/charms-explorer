//! Helper functions for asset repository

use crate::domain::models::Asset;
use crate::infrastructure::persistence::entities::assets;

/// Extract identity hash from app_id
/// Format: "{tag}/{identity}/{vk}" -> returns "{identity}"
/// Example: "t/abc123.../def456..." -> "abc123..."
pub fn extract_hash_from_app_id(app_id: &str) -> String {
    // Split by '/' and get the identity part (second element)
    let parts: Vec<&str> = app_id.split('/').collect();
    if parts.len() >= 2 {
        parts[1].to_string()
    } else {
        app_id.to_string()
    }
}

/// Convert database entity to domain model
pub fn to_domain_model(entity: assets::Model) -> Asset {
    Asset::new(
        entity.app_id,
        entity.txid,
        entity.vout_index,
        entity.charm_id,
        entity.block_height as u64,
        entity.date_created.naive_utc(),
        entity.data,
        entity.asset_type,
        entity.blockchain,
        entity.network,
    )
}
