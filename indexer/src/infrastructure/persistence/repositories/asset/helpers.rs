//! Helper functions for asset repository

use crate::domain::models::Asset;
use crate::infrastructure::persistence::entities::assets;

/// Extract hash from app_id (removes t/ or n/ prefix and returns only the hash part)
/// Example: "t/3d7f.../txid:vout" -> "3d7f..." (64 char hash only)
pub fn extract_hash_from_app_id(app_id: &str) -> String {
    let without_prefix = if app_id.starts_with("t/") || app_id.starts_with("n/") {
        &app_id[2..]
    } else {
        app_id
    };

    // Hash is 64 characters, extract only that part
    if without_prefix.len() >= 64 {
        without_prefix[..64].to_string()
    } else {
        without_prefix.to_string()
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
