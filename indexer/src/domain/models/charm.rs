use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a Charm asset found in a blockchain transaction
/// [RJJ-S01] Updated: Removed charmid field, added app_id field
/// [RJJ-MEMPOOL] Updated: block_height is now Option<u64> to support mempool charms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Charm {
    /// Transaction ID
    pub txid: String,

    /// Output index (vout) where the charm is located in the transaction
    pub vout: i32,

    /// Block height where the charm was found (None if still in mempool)
    pub block_height: Option<u64>,

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

    /// Bitcoin address that holds the charm
    pub address: Option<String>,

    /// Whether the UTXO has been spent
    pub spent: bool,

    /// App ID of the charm (t/xxx for tokens, n/xxx for NFTs, or "other")
    pub app_id: String,

    /// Amount of the charm (with 8 decimals, stored as satoshis/units)
    pub amount: i64,

    /// Timestamp when charm was first detected in mempool (None if indexed directly from block)
    pub mempool_detected_at: Option<NaiveDateTime>,

    /// Tags for categorization (comma-separated, e.g., "charms-cast,create-ask")
    pub tags: Option<String>,
}

impl Charm {
    /// Creates a new Charm with specified parameters
    /// [RJJ-S01] Updated: Removed charmid parameter, added app_id and amount
    /// [RJJ-MEMPOOL] Updated: block_height is now Option<u64>, added mempool_detected_at
    pub fn new(
        txid: String,
        vout: i32,
        block_height: Option<u64>,
        data: Value,
        date_created: NaiveDateTime,
        asset_type: String,
        blockchain: String,
        network: String,
        address: Option<String>,
        spent: bool,
        app_id: String,
        amount: i64,
    ) -> Self {
        Self {
            txid,
            vout,
            block_height,
            data,
            date_created,
            asset_type,
            blockchain,
            network,
            address,
            spent,
            app_id,
            amount,
            mempool_detected_at: None, // Will be set by repository if charm is from mempool
            tags: None,
        }
    }

    /// Creates a new Charm with tags
    pub fn with_tags(mut self, tags: Option<String>) -> Self {
        self.tags = tags;
        self
    }
}
