//! Data structures for charm save requests

use serde_json::Value;

/// Request to save a transaction to the database
#[derive(Debug, Clone)]
pub struct TransactionSaveRequest {
    pub txid: String,
    pub block_height: u64,
    pub tx_position: i64,
    pub raw_hex: String,
    pub confirmations: i32,
    pub is_confirmed: bool,
    pub blockchain: String,
    pub network: String,
}

/// Request to save an asset to the database
#[derive(Debug, Clone)]
pub struct AssetSaveRequest {
    pub app_id: String,
    pub asset_type: String,
    pub supply: u64,
    pub blockchain: String,
    pub network: String,
    // Metadata fields (optional, extracted from NFT)
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub decimals: Option<u8>,
}

/// Request to save a charm to the database
/// [RJJ-S01] Removed charmid field, added app_id and amount
/// [RJJ-DEX] Added tags field for product tagging
#[derive(Debug, Clone)]
pub struct CharmSaveRequest {
    pub txid: String,
    pub vout: i32,
    pub block_height: u64,
    pub data: Value,
    pub asset_type: String,
    pub blockchain: String,
    pub network: String,
    pub address: Option<String>,
    pub tx_position: i64,
    pub app_id: String,
    pub amount: i64,
    pub tags: Option<String>,
    pub spent: bool,
}
