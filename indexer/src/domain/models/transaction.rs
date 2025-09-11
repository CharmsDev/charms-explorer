use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a blockchain transaction with charm data
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

    /// Blockchain type (e.g., "Bitcoin", "Cardano")
    pub blockchain: String,

    /// Network name (e.g., "mainnet", "testnet4")
    pub network: String,
}

impl Transaction {
    /// Creates a new Transaction with specified parameters
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
        blockchain: String,
        network: String,
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
            blockchain,
            network,
        }
    }

    /// Returns true if transaction has sufficient confirmations
    pub fn is_confirmed(&self) -> bool {
        self.confirmations >= 6 || self.status == "confirmed"
    }
}
