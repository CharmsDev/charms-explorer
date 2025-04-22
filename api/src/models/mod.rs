// API request/response models
use serde::{Deserialize, Serialize};

/// Query parameters for GET /charms/count endpoint
#[derive(Debug, Deserialize)]
pub struct GetCharmNumbersQuery {
    #[serde(rename = "type")]
    pub asset_type: Option<String>,
}

/// Response structure for GET /charms/count endpoint
#[derive(Debug, Serialize)]
pub struct CharmCountResponse {
    pub count: usize,
}

/// Response structure for GET /charms endpoint
#[derive(Debug, Serialize)]
pub struct CharmsResponse {
    pub charms: Vec<CharmData>,
}

/// Charm data structure for API responses
#[derive(Debug, Serialize)]
pub struct CharmData {
    pub txid: String,
    pub charmid: String,
    pub block_height: i32,
    pub data: serde_json::Value,
    pub date_created: String,
    pub asset_type: String,
}

/// Query parameters for GET /charms/by-type endpoint
#[derive(Debug, Deserialize)]
pub struct GetCharmsByTypeQuery {
    #[serde(rename = "type")]
    pub asset_type: String,
}
