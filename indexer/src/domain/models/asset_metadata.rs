/// [RJJ-DECIMALS] Asset metadata including decimal precision
/// 
/// This module handles dynamic decimal precision for token amounts based on NFT metadata.
/// 
/// ## Decimal Precision Rules:
/// 1. If NFT exists with `decimals` metadata → use that value
/// 2. If no NFT exists → assume 8 decimals (Bitcoin standard)
/// 3. If NFT exists but no `decimals` field → assume 8 decimals
/// 
/// ## Example:
/// ```json
/// {
///   "decimals": 8,
///   "name": "Ebro Token",
///   "symbol": "EBRO"
/// }
/// ```

use serde::{Deserialize, Serialize};

/// Default number of decimals for tokens (Bitcoin standard)
pub const DEFAULT_DECIMALS: u8 = 8;

/// Asset metadata extracted from NFT charm data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadata {
    /// Number of decimal places for token amounts
    /// [RJJ-DECIMALS] This value is extracted from the NFT metadata
    pub decimals: u8,
    
    /// Asset name (optional)
    pub name: Option<String>,
    
    /// Asset symbol (optional)
    pub symbol: Option<String>,
    
    /// Asset description (optional)
    pub description: Option<String>,
    
    /// Image URL (optional)
    pub image_url: Option<String>,
}

impl Default for AssetMetadata {
    fn default() -> Self {
        Self {
            decimals: DEFAULT_DECIMALS,
            name: None,
            symbol: None,
            description: None,
            image_url: None,
        }
    }
}

impl AssetMetadata {
    /// Extract metadata from NFT charm data
    /// [RJJ-DECIMALS] Looks for 'decimals' field in the data
    pub fn from_nft_data(data: &serde_json::Value) -> Self {
        let mut metadata = Self::default();
        
        // Try to extract from data.data field
        if let Some(data_section) = data.get("data") {
            if let Some(obj) = data_section.as_object() {
                // Extract decimals
                if let Some(decimals) = obj.get("decimals") {
                    if let Some(d) = decimals.as_u64() {
                        metadata.decimals = d.min(18) as u8; // Cap at 18 decimals for safety
                    }
                }
                
                // Extract name
                if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                    metadata.name = Some(name.to_string());
                }
                
                // Extract symbol
                if let Some(symbol) = obj.get("symbol").and_then(|v| v.as_str()) {
                    metadata.symbol = Some(symbol.to_string());
                }
                
                // Extract description
                if let Some(desc) = obj.get("description").and_then(|v| v.as_str()) {
                    metadata.description = Some(desc.to_string());
                }
                
                // Extract image_url
                if let Some(image) = obj.get("image").and_then(|v| v.as_str()) {
                    metadata.image_url = Some(image.to_string());
                } else if let Some(image_url) = obj.get("image_url").and_then(|v| v.as_str()) {
                    metadata.image_url = Some(image_url.to_string());
                }
            }
        }
        
        metadata
    }
    
    /// Extract hash from app_id (removes t/ or n/ prefix)
    /// [RJJ-DECIMALS] Used to match tokens with their reference NFT
    pub fn extract_hash_from_app_id(app_id: &str) -> String {
        let parts: Vec<&str> = app_id.split('/').collect();
        if parts.len() >= 2 {
            parts[1..].join("/")
        } else {
            app_id.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_default_decimals() {
        let metadata = AssetMetadata::default();
        assert_eq!(metadata.decimals, 8);
    }

    #[test]
    fn test_extract_decimals_from_nft() {
        let data = json!({
            "data": {
                "decimals": 6,
                "name": "Test Token",
                "symbol": "TEST"
            }
        });
        
        let metadata = AssetMetadata::from_nft_data(&data);
        assert_eq!(metadata.decimals, 6);
        assert_eq!(metadata.name, Some("Test Token".to_string()));
        assert_eq!(metadata.symbol, Some("TEST".to_string()));
    }

    #[test]
    fn test_extract_hash_from_app_id() {
        let app_id = "t/abc123/def456";
        let hash = AssetMetadata::extract_hash_from_app_id(app_id);
        assert_eq!(hash, "abc123/def456");
        
        let nft_id = "n/abc123/def456";
        let nft_hash = AssetMetadata::extract_hash_from_app_id(nft_id);
        assert_eq!(nft_hash, "abc123/def456");
    }
}
