use anyhow::Result;
use charms_client::tx::{extract_and_verify_spell, Tx};
use charms_client::NormalizedSpell;

/// Native charm parser using the charms-client crate
/// Provides direct parsing and verification of charm transactions
pub struct NativeCharmParser;

impl NativeCharmParser {
    /// Spell verification key for the current protocol version
    pub const SPELL_VK: &'static str =
        "0x0041d9843ec25ba04797a0ce29af364389f7eda9f7126ef39390c357432ad9aa";

    /// Extract and verify a charm from a transaction hex string
    ///
    /// This function:
    /// 1. Parses the transaction hex into a Tx object
    /// 2. Extracts the spell data from the transaction
    /// 3. Verifies the cryptographic proof
    /// 4. Returns the normalized spell data
    ///
    /// Returns Ok(NormalizedSpell) if the transaction contains a valid charm
    /// Returns Err if the transaction is invalid or doesn't contain a charm
    pub fn extract_and_verify_charm(tx_hex: &str, mock: bool) -> Result<NormalizedSpell> {
        // Parse transaction from hex
        let tx = Tx::from_hex(tx_hex)?;

        // Extract and verify spell using the charms-client library
        let normalized_spell = extract_and_verify_spell(Self::SPELL_VK, &tx, mock)?;

        Ok(normalized_spell)
    }

    /// Check if a transaction hex could potentially be a charm
    /// This is a lightweight check before doing full parsing
    pub fn could_be_charm(tx_hex: &str) -> bool {
        // Try to parse as transaction first
        if let Ok(_tx) = Tx::from_hex(tx_hex) {
            // Try to extract spell - if it succeeds, it's a charm
            Self::extract_and_verify_charm(tx_hex, false).is_ok()
        } else {
            false
        }
    }

    /// Extract asset-related data from a normalized spell
    /// Returns information that can be used to populate the assets table
    pub fn extract_asset_info(spell: &NormalizedSpell) -> Vec<AssetInfo> {
        let mut assets = Vec::new();

        // Extract information from spell outputs (NormalizedCharms)
        for (output_index, normalized_charms) in spell.tx.outs.iter().enumerate() {
            // Each output can contain multiple charms
            for (app_index, charm_data) in normalized_charms.iter() {
                // Get the app from the spell's app_public_inputs
                if let Some((app, _)) = spell.app_public_inputs.iter().nth(*app_index as usize) {
                    let asset_info = AssetInfo {
                        app_id: format!("{}:{}", app, output_index),
                        vout_index: output_index as i32,
                        amount: extract_amount_from_charm_data(charm_data),
                        asset_type: determine_asset_type_from_app(app),
                    };
                    assets.push(asset_info);
                }
            }
        }

        assets
    }
}

/// Information extracted from a charm that's relevant for the assets table
#[derive(Debug, Clone)]
pub struct AssetInfo {
    pub app_id: String,
    pub vout_index: i32,
    pub amount: u64,
    pub asset_type: String,
}

/// Extract amount from charm data
fn extract_amount_from_charm_data(charm_data: &charms_data::Data) -> u64 {
    // Try to extract amount from charm data as u64
    // This is a simplified implementation - can be enhanced based on actual data structure
    if let Ok(amount) = charm_data.value::<u64>() {
        // Cap at i64::MAX to prevent overflow when storing in database
        amount.min(i64::MAX as u64)
    } else if let Ok(amount) = charm_data.value::<i64>() {
        amount.max(0) as u64
    } else {
        // Try to extract from bytes representation
        let bytes = charm_data.bytes();
        if bytes.len() >= 8 {
            let raw_amount = u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]);
            // Cap at i64::MAX to prevent overflow when storing in database
            raw_amount.min(i64::MAX as u64)
        } else {
            0
        }
    }
}

/// Determine the asset type based on app information
fn determine_asset_type_from_app(app: &charms_data::App) -> String {
    // Logic to determine asset type based on app tag
    // This follows the charm protocol rules for asset types
    match app.tag {
        charms_data::TOKEN => "token".to_string(),
        charms_data::NFT => "nft".to_string(),
        _ => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spell_vk_constant() {
        assert!(!NativeCharmParser::SPELL_VK.is_empty());
        assert!(NativeCharmParser::SPELL_VK.starts_with("0x"));
    }

    #[test]
    fn test_could_be_charm_invalid_hex() {
        let invalid_hex = "invalid_hex_string";
        assert!(!NativeCharmParser::could_be_charm(invalid_hex));
    }

    #[test]
    fn test_could_be_charm_regular_transaction() {
        // Regular Bitcoin transaction without charm data
        let regular_tx = "0200000001f2b3eb2deb76566e7324307cd47c35eeb88413f971d88519859b1834307ecfec01000000006a47304402203ad1cc746a3cb70ca10e7e3612a9370b8b5d4c8b3b5c7b5c7b5c7b5c7b5c7b5c02203ad1cc746a3cb70ca10e7e3612a9370b8b5d4c8b3b5c7b5c7b5c7b5c7b5c7b5c0121025476c2e83188368da1ff3e292e7acafcdb3566bb0ad253f62fc70f07aeee6357ffffffff0100e1f50500000000196a17a91489abcdefabbaabbaabbaabbaabbaabbaabbaabba8700000000";
        assert!(!NativeCharmParser::could_be_charm(regular_tx));
    }
}
