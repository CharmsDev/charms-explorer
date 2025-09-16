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
}
