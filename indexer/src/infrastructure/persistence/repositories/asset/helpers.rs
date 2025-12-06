//! Helper functions for asset repository

use crate::domain::models::Asset;
use crate::infrastructure::persistence::entities::assets;

/// Extract hash from app_id (removes t/ or n/ prefix)
/// Example: "t/3d7f.../..." -> "3d7f.../..."
pub fn extract_hash_from_app_id(app_id: &str) -> String {
    if app_id.starts_with("t/") || app_id.starts_with("n/") {
        app_id[2..].to_string()
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
