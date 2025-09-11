use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a Charm asset found in a blockchain transaction
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

    /// Blockchain type (e.g., "Bitcoin", "Cardano")
    pub blockchain: String,

    /// Network name (e.g., "mainnet", "testnet4")
    pub network: String,
}

impl Charm {
    /// Creates a new Charm with specified parameters
    pub fn new(
        txid: String,
        charmid: String,
        block_height: u64,
        data: Value,
        date_created: NaiveDateTime,
        asset_type: String,
        blockchain: String,
        network: String,
    ) -> Self {
        Self {
            txid,
            charmid,
            block_height,
            data,
            date_created,
            asset_type,
            blockchain,
            network,
        }
    }
}
