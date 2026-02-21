// API request/response models
use serde::{Deserialize, Deserializer, Serialize};

/// Custom deserializer to convert string to u64
fn deserialize_string_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    s.parse::<u64>().map_err(serde::de::Error::custom)
}

/// Common pagination parameters for API endpoints
#[derive(Debug, Deserialize, Default)]
pub struct PaginationParams {
    #[serde(
        default = "default_page",
        deserialize_with = "deserialize_string_to_u64"
    )]
    pub page: u64,
    #[serde(
        default = "default_limit",
        deserialize_with = "deserialize_string_to_u64"
    )]
    pub limit: u64,
    #[serde(default = "default_sort_order")]
    #[allow(dead_code)] // Deserialized from query but not yet implemented
    pub sort: String,
}

fn default_page() -> u64 {
    1
}

fn default_limit() -> u64 {
    20
}

fn default_sort_order() -> String {
    "newest".to_string()
}

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

/// Response structure for GET /charms/count-by-type endpoint
#[derive(Debug, Serialize)]
pub struct CharmsCountByTypeResponse {
    pub total: u64,
    pub nft: u64,
    pub token: u64,
    pub dapp: u64,
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
    pub vout: i32, // [RJJ-ADDRESS] Output index for UTXO identification
    pub charmid: String,
    pub block_height: Option<i32>,
    pub data: serde_json::Value,
    pub date_created: String,
    pub asset_type: String,
    pub network: String,
    pub amount: i64, // [RJJ-ADDRESS] Token amount in this UTXO
    #[serde(default = "default_likes_count")]
    pub likes_count: i64,
    #[serde(default = "default_user_liked")]
    pub user_liked: bool,
    // [RJJ-METADATA] Enriched metadata from assets table
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the spell proof has been verified
    pub verified: bool,
    // [RJJ-BEAMING] Tags for transaction classification (e.g., "beaming", "bro", "charms-cast")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
    // [RJJ-SPELL] Original spell data from transactions table
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spell: Option<serde_json::Value>,
}

#[allow(dead_code)] // Used by serde default attribute
fn default_likes_count() -> i64 {
    0
}

#[allow(dead_code)] // Used by serde default attribute
fn default_user_liked() -> bool {
    false
}

/// Query parameters for GET /charms/by-type endpoint
#[derive(Debug, Deserialize)]
pub struct GetCharmsByTypeQuery {
    #[serde(rename = "type")]
    pub asset_type: String,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Query parameters for GET /charms endpoint
#[derive(Debug, Deserialize, Default)]
pub struct GetCharmsQuery {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(default = "default_user_id")]
    pub user_id: i32,
    pub network: Option<String>,
}

fn default_user_id() -> i32 {
    1 // Default user ID as specified in requirements
}

/// Request body for POST /charms/like endpoint
#[derive(Debug, Deserialize)]
pub struct LikeCharmRequest {
    pub charm_id: String,
    #[serde(default = "default_user_id")]
    pub user_id: i32,
}

/// Response structure for like operations
#[derive(Debug, Serialize)]
pub struct LikeResponse {
    pub success: bool,
    pub message: String,
    pub likes_count: i64,
}

/// Pagination metadata for responses
#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub total: u64,
    pub page: u64,
    pub limit: u64,
    pub total_pages: u64,
}

/// Response structure with pagination
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: T,
    pub pagination: PaginationMeta,
}
