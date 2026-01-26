//! DEX types for Charms Cast order detection and parsing

use serde::{Deserialize, Serialize};

/// Known DEX contract verification keys
pub mod dex_vks {
    /// Charms Cast DEX v0.1 (legacy)
    pub const CAST_V01: &str = "ce0c45fe29f26ff197bf9288e62ad7513941294d513e724854d97bee53e03a45";
    /// Charms Cast DEX v0.2 (current)
    pub const CAST_V02: &str = "a471d3fcc436ae7cbc0e0c82a68cdc8e003ee21ef819e1acf834e11c43ce47d8";
}

/// Known token identifiers for tagging
pub mod known_tokens {
    /// $BRO token - identity hash (the part after n/ or t/ and before the VK)
    pub const BRO_IDENTITY_1: &str =
        "3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b";
    pub const BRO_IDENTITY_2: &str =
        "6274399ab68d4a35e5193394aded0bed548453f6ebb7ea46dd2ca0c251f74580";
}

/// Check if an app_id is the $BRO token
pub fn is_bro_token(app_id: &str) -> bool {
    app_id.contains(known_tokens::BRO_IDENTITY_1) || app_id.contains(known_tokens::BRO_IDENTITY_2)
}

/// Type of DEX operation detected in a transaction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DexOperation {
    /// Create a sell order (offer tokens for BTC)
    CreateAskOrder,
    /// Create a buy order (offer BTC for tokens)
    CreateBidOrder,
    /// Fulfill an ask order (buy tokens with BTC)
    FulfillAsk,
    /// Fulfill a bid order (sell tokens for BTC)
    FulfillBid,
    /// Cancel an existing order
    CancelOrder,
    /// Partially fill an order
    PartialFill,
}

impl DexOperation {
    /// Returns the tag string for this operation
    pub fn to_tag(&self) -> &'static str {
        match self {
            DexOperation::CreateAskOrder => "create-ask",
            DexOperation::CreateBidOrder => "create-bid",
            DexOperation::FulfillAsk => "fulfill-ask",
            DexOperation::FulfillBid => "fulfill-bid",
            DexOperation::CancelOrder => "cancel",
            DexOperation::PartialFill => "partial-fill",
        }
    }
}

/// Order side: buy or sell
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    /// Ask order: selling tokens for BTC
    Ask,
    /// Bid order: buying tokens with BTC
    Bid,
}

/// Execution type for orders
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecType {
    /// Order must be filled completely or not at all
    AllOrNone,
    /// Order can be partially filled
    Partial {
        /// Reference to parent order if this is a remainder
        from: Option<String>,
    },
}

/// Represents a DEX order extracted from a spell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexOrder {
    /// Bitcoin address of the order maker
    pub maker: String,
    /// Order side (ask/bid)
    pub side: OrderSide,
    /// Execution type
    pub exec_type: ExecType,
    /// Price as [numerator, denominator]
    pub price: (u64, u64),
    /// Amount in satoshis
    pub amount: u64,
    /// Quantity in token units (with decimals)
    pub quantity: u64,
    /// Asset app_id (token being traded)
    pub asset_app_id: String,
    /// Scrolls address where order is held (if applicable)
    pub scrolls_address: Option<String>,
}

impl DexOrder {
    /// Calculate price per token (sats per whole token, assuming 8 decimals)
    pub fn price_per_token(&self) -> f64 {
        if self.price.1 == 0 {
            return 0.0;
        }
        // price = num/den sats per unit
        // For 8 decimal token: multiply by 10^8 to get sats per whole token
        (self.price.0 as f64 / self.price.1 as f64) * 100_000_000.0
    }
}

/// Result of DEX detection on a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexDetectionResult {
    /// Type of operation detected
    pub operation: DexOperation,
    /// DEX contract app_id used
    pub dex_app_id: String,
    /// Order data (if applicable)
    pub order: Option<DexOrder>,
    /// Input order IDs consumed (for fulfills/cancels)
    pub input_order_ids: Vec<String>,
    /// Output order ID created (for creates/partial fills)
    pub output_order_id: Option<String>,
    /// Tags to apply to the transaction
    pub tags: Vec<String>,
}

impl DexDetectionResult {
    /// Create tags string for database storage
    pub fn tags_string(&self) -> String {
        self.tags.join(",")
    }
}

/// Check if an app_id is a known DEX contract
pub fn is_dex_app_id(app_id: &str) -> bool {
    // DEX app_id format: b/0000...0000/<vk>
    if !app_id.starts_with("b/") {
        return false;
    }

    // Check for identity = 0 (64 zeros)
    let zero_identity = "0".repeat(64);
    if !app_id.contains(&format!("/{}/", zero_identity))
        && !app_id.starts_with(&format!("b/{}/", zero_identity))
    {
        return false;
    }

    // Check for known VKs
    app_id.ends_with(dex_vks::CAST_V01) || app_id.ends_with(dex_vks::CAST_V02)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_dex_app_id() {
        let valid_v02 = "b/0000000000000000000000000000000000000000000000000000000000000000/a471d3fcc436ae7cbc0e0c82a68cdc8e003ee21ef819e1acf834e11c43ce47d8";
        let valid_v01 = "b/0000000000000000000000000000000000000000000000000000000000000000/ce0c45fe29f26ff197bf9288e62ad7513941294d513e724854d97bee53e03a45";
        let token_app = "t/3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b/c975d4e0c292fb95efbda5c13312d6ac1d8b5aeff7f0f1e5578645a2da70ff5f";
        let unknown_b =
            "b/0000000000000000000000000000000000000000000000000000000000000000/unknown_vk";

        assert!(is_dex_app_id(valid_v02));
        assert!(is_dex_app_id(valid_v01));
        assert!(!is_dex_app_id(token_app));
        assert!(!is_dex_app_id(unknown_b));
    }

    #[test]
    fn test_operation_tags() {
        assert_eq!(DexOperation::CreateAskOrder.to_tag(), "create-ask");
        assert_eq!(DexOperation::FulfillBid.to_tag(), "fulfill-bid");
        assert_eq!(DexOperation::PartialFill.to_tag(), "partial-fill");
    }
}
