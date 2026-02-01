//! Common types for reindexer batch processing

use serde_json::Value;

/// Spell data for batch insert
pub type SpellBatch = Vec<(
    String,       // txid
    u64,          // block_height
    Value,        // data
    String,       // blockchain
    String,       // network
)>;

/// Charm data for batch insert
pub type CharmBatch = Vec<(
    String,       // txid
    i32,          // vout
    u64,          // block_height
    Value,        // data
    String,       // asset_type
    String,       // blockchain
    String,       // network
    Option<String>, // address
    String,       // app_id
    i64,          // amount
    Option<String>, // spent_by
)>;

/// Asset data for batch insert
pub type AssetBatch = Vec<(
    String,       // app_id
    String,       // txid
    i32,          // vout
    String,       // charm_id
    u64,          // block_height
    Value,        // data
    String,       // asset_type
    String,       // blockchain
    String,       // network
)>;

/// Stats holder update data (app_id, address, amount_delta, block_height)
pub type HolderUpdate = (String, String, i64, i32);

/// Convert token app_id (t/HASH) to NFT app_id (n/HASH) for stats consolidation
#[inline]
pub fn to_nft_app_id(app_id: String) -> String {
    if app_id.starts_with("t/") {
        app_id.replacen("t/", "n/", 1)
    } else {
        app_id
    }
}
