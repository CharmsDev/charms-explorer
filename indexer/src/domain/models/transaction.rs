use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Domain model for a Bitcoin transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction ID
    pub txid: String,

    /// Block height where the transaction was found
    pub block_height: u64,

    /// Position of the transaction in the block
    pub ordinal: i64,

    /// Raw transaction data
    pub raw: Value,

    /// Charm data if this transaction contains a charm
    pub charm: Value,

    /// Last update time
    pub updated_at: NaiveDateTime,

    /// Number of confirmations
    pub confirmations: i32,

    /// Status of the transaction (pending, confirmed, etc.)
    pub status: String,
}

impl Transaction {
    /// Create a new Transaction
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        txid: String,
        block_height: u64,
        ordinal: i64,
        raw: Value,
        charm: Value,
        updated_at: NaiveDateTime,
        confirmations: i32,
        status: String,
    ) -> Self {
        Self {
            txid,
            block_height,
            ordinal,
            raw,
            charm,
            updated_at,
            confirmations,
            status,
        }
    }

    /// Check if the transaction is confirmed
    pub fn is_confirmed(&self) -> bool {
        self.confirmations >= 6 || self.status == "confirmed"
    }
}
