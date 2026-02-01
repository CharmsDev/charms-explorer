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

/// Normalize image value - handles both URLs and base64 data
/// People sometimes put URLs in the 'image' field instead of 'image_url'
/// This function detects the type and returns the value as-is (both are valid for display)
fn normalize_image_value(value: &str) -> String {
    let trimmed = value.trim();

    // Already a data URI (base64) - return as-is
    if trimmed.starts_with("data:") {
        return trimmed.to_string();
    }

    // HTTP/HTTPS URL - return as-is
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }

    // IPFS URL - return as-is
    if trimmed.starts_with("ipfs://") {
        return trimmed.to_string();
    }

    // If it looks like base64 without the data: prefix, try to detect and fix
    // Base64 typically contains only alphanumeric chars, +, /, and =
    if trimmed.len() > 100 && !trimmed.contains(' ') && !trimmed.contains('/') {
        // Likely raw base64 without prefix - assume PNG
        return format!("data:image/png;base64,{}", trimmed);
    }

    // Unknown format - return as-is and let the frontend handle it
    trimmed.to_string()
}

impl AssetMetadata {
    /// Extract metadata from NFT charm data
    /// Looks in both top-level and data.data for metadata fields
    pub fn from_nft_data(data: &serde_json::Value) -> Self {
        let mut metadata = Self::default();

        // Helper to extract from an object
        fn extract_from_obj(
            obj: &serde_json::Map<String, serde_json::Value>,
            metadata: &mut AssetMetadata,
        ) {
            if let Some(decimals) = obj.get("decimals").and_then(|v| v.as_u64()) {
                metadata.decimals = decimals.min(18) as u8;
            }
            if metadata.name.is_none() {
                if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                    metadata.name = Some(name.to_string());
                }
            }
            if metadata.symbol.is_none() {
                if let Some(symbol) = obj.get("symbol").and_then(|v| v.as_str()) {
                    metadata.symbol = Some(symbol.to_string());
                }
            }
            if metadata.description.is_none() {
                if let Some(desc) = obj.get("description").and_then(|v| v.as_str()) {
                    metadata.description = Some(desc.to_string());
                }
            }
            if metadata.image_url.is_none() {
                if let Some(image) = obj.get("image").and_then(|v| v.as_str()) {
                    metadata.image_url = Some(normalize_image_value(image));
                } else if let Some(image_url) = obj.get("image_url").and_then(|v| v.as_str()) {
                    metadata.image_url = Some(normalize_image_value(image_url));
                }
            }
        }

        // First try top-level (for batch-saved assets)
        if let Some(obj) = data.as_object() {
            extract_from_obj(obj, &mut metadata);
        }

        // Then try data.data field (for raw charm data)
        // The charm data structure is: {"app_id": "...", "data": {"name": "Bro", ...}, "type": "charm", "asset_type": "nft"}
        if let Some(data_section) = data.get("data") {
            // Check if data_section is null (common case for tokens)
            if !data_section.is_null() {
                if let Some(obj) = data_section.as_object() {
                    extract_from_obj(obj, &mut metadata);
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
