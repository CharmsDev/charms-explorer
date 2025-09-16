use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents an Asset with a unique app_id that can have multiple charms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    /// Unique application identifier (e.g., n/..., t/..., etc.)
    pub app_id: String,

    /// Transaction ID where this asset exists
    pub txid: String,

    /// UTXO index (vout index)
    pub vout_index: i32,

    /// Charm ID that references this asset
    pub charm_id: String,

    /// Block height where the asset was found
    pub block_height: u64,

    /// Date when the asset was created
    pub date_created: NaiveDateTime,

    /// JSON data associated with the asset
    pub data: Value,

    /// Type of asset
    pub asset_type: String,

    /// Blockchain type (e.g., "Bitcoin", "Cardano")
    pub blockchain: String,

    /// Network name (e.g., "mainnet", "testnet4")
    pub network: String,
}

impl Asset {
    /// Creates a new Asset with specified parameters
    pub fn new(
        app_id: String,
        txid: String,
        vout_index: i32,
        charm_id: String,
        block_height: u64,
        date_created: NaiveDateTime,
        data: Value,
        asset_type: String,
        blockchain: String,
        network: String,
    ) -> Self {
        Self {
            app_id,
            txid,
            vout_index,
            charm_id,
            block_height,
            date_created,
            data,
            asset_type,
            blockchain,
            network,
        }
    }
}
