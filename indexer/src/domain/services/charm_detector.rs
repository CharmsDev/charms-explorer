use serde_json::Value;

const SPELL_BYTES: &[u8] = b"spell";

/// Detects and analyzes charm transactions in blockchain data
pub struct CharmDetectorService;

impl CharmDetectorService {
    /// Checks if transaction contains the "spell" marker
    /// Now accepts context parameters for better logging
    pub fn could_be_charm_with_context(
        tx_hex: &str,
        txid: &str,
        block_height: u64,
        tx_pos: usize,
        network_name: &str,
    ) -> bool {
        if let Ok(tx_bytes) = hex::decode(tx_hex) {
            // Look for the "spell" ASCII string in the transaction
            for window in tx_bytes.windows(SPELL_BYTES.len()) {
                if window == SPELL_BYTES {
                    crate::utils::logging::log_info(&format!(
                        "[{}] 🎯 Block {}: Found 'spell' marker in tx {} at position {}",
                        network_name, block_height, txid, tx_pos
                    ));
                    return true;
                }
            }
        } else {
            crate::utils::logging::log_error(&format!(
                "[{}] ❌ Block {}: Failed to decode transaction hex for tx {} at position {}",
                network_name, block_height, txid, tx_pos
            ));
        }

        false
    }

    /// Legacy method for backward compatibility
    pub fn could_be_charm(tx_hex: &str) -> bool {
        if let Ok(tx_bytes) = hex::decode(tx_hex) {
            // Look for the "spell" ASCII string in the transaction
            for window in tx_bytes.windows(SPELL_BYTES.len()) {
                if window == SPELL_BYTES {
                    return true;
                }
            }
        }
        false
    }

    /// Analyzes transaction to determine if it's a charm
    pub fn analyze_charm_transaction(tx_hex: &str) -> Option<()> {
        if Self::could_be_charm(tx_hex) {
            Some(())
        } else {
            None
        }
    }

    /// Processes charm data from the API
    pub fn process_spell_data(spell_data: Value) -> Value {
        spell_data
    }

    /// Extracts app_id from charm API response
    /// Returns the first app_id found in the charm data, or None if not found
    pub fn extract_app_id_from_spell_data(spell_data: &Value) -> Option<String> {
        // Look for app_id in the API response structure
        if let Some(outs) = spell_data.get("outs").and_then(|v| v.as_array()) {
            for out in outs {
                if let Some(charms) = out.get("charms").and_then(|v| v.as_object()) {
                    // Look for app_id in charm data - it could be in various formats
                    // Common patterns: "n/...", "t/...", etc.
                    for (key, value) in charms {
                        if let Some(charm_data) = value.as_object() {
                            // Check if this charm has an app_id field
                            if let Some(app_id) = charm_data.get("app_id").and_then(|v| v.as_str()) {
                                return Some(app_id.to_string());
                            }
                            // Check if the key itself is an app_id pattern
                            if key.starts_with("n/") || key.starts_with("t/") || key.starts_with("r/") {
                                return Some(key.clone());
                            }
                        }
                        // If the value is a string and looks like an app_id
                        if let Some(app_id_str) = value.as_str() {
                            if app_id_str.starts_with("n/") || app_id_str.starts_with("t/") || app_id_str.starts_with("r/") {
                                return Some(app_id_str.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback: look for app_id at the root level
        if let Some(app_id) = spell_data.get("app_id").and_then(|v| v.as_str()) {
            return Some(app_id.to_string());
        }
        
        None
    }
}
