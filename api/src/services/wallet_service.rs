// Wallet response types.
// Live chain data comes from the Esplora client (mempool_space_service).

use serde::{Deserialize, Serialize};

// --- Response types ---

#[derive(Debug, Serialize, Deserialize)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub value: u64,
    pub script_pubkey: String,
    pub confirmations: u32,
    /// Block height at which this UTXO confirmed. `None` (or 0) = mempool.
    /// Esplora providers expose `status.block_height` for confirmed outputs;
    /// it is the source of truth, not `confirmations` (which is a derived count).
    #[serde(default)]
    pub block_height: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionDetail {
    pub txid: String,
    pub version: i32,
    pub locktime: u32,
    pub size: usize,
    pub vsize: usize,
    pub weight: usize,
    pub fee: Option<f64>,
    pub confirmations: Option<u32>,
    pub block_hash: Option<String>,
    pub block_height: Option<u32>,
    pub time: Option<u64>,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub hex: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxInput {
    pub txid: String,
    pub vout: u32,
    pub script_sig: String,
    pub sequence: u32,
    pub witness: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxOutput {
    pub value: f64,
    pub n: u32,
    pub script_pubkey: ScriptPubKey,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptPubKey {
    pub asm: String,
    pub hex: String,
    #[serde(rename = "type")]
    pub script_type: String,
    pub address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeeEstimate {
    pub fee_rate: f64,
    pub blocks: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainTip {
    pub height: u64,
    pub hash: String,
    pub time: Option<u64>,
}

/// An address-history record from Esplora /address/:a/txs (used for seeding)
#[derive(Debug, Serialize, Deserialize)]
pub struct AddressTxRecord {
    pub txid: String,
    pub direction: String,
    pub amount: i64,
    pub fee: i64,
    pub block_height: Option<i32>,
    pub block_time: Option<i64>,
    pub confirmations: i32,
}
