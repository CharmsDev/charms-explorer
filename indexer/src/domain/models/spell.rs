// [RJJ-S01] New Spell domain model
// A Spell is the container in output 0 that describes all charms in a transaction
// Spells are OP_RETURN outputs and don't have addresses (only charms have addresses)

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a Spell found in output 0 of a charm transaction
/// The spell contains metadata about all charms in the transaction
/// Note: Spells are OP_RETURN outputs and don't have addresses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spell {
    /// Transaction ID where the spell is located (output 0)
    pub txid: String,
    pub block_height: u64,
    pub data: Value,
    pub date_created: NaiveDateTime,
    pub asset_type: String,
    pub blockchain: String,
    pub network: String,
}

impl Spell {
    pub fn new(
        txid: String,
        block_height: u64,
        data: Value,
        date_created: NaiveDateTime,
        asset_type: String,
        blockchain: String,
        network: String,
    ) -> Self {
        Self {
            txid,
            block_height,
            data,
            date_created,
            asset_type,
            blockchain,
            network,
        }
    }
}
