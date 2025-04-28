use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Domain model for a Charm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Charm {
    /// Transaction ID
    pub txid: String,

    /// Charm ID
    pub charmid: String,

    /// Block height where the charm was found
    pub block_height: u64,

    /// JSON data associated with the charm
    pub data: Value,

    /// Date when the charm was created
    pub date_created: NaiveDateTime,

    /// Type of asset
    pub asset_type: String,
}

impl Charm {
    /// Create a new Charm
    pub fn new(
        txid: String,
        charmid: String,
        block_height: u64,
        data: Value,
        date_created: NaiveDateTime,
        asset_type: String,
    ) -> Self {
        Self {
            txid,
            charmid,
            block_height,
            data,
            date_created,
            asset_type,
        }
    }
}
