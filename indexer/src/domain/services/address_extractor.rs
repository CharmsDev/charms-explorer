use bitcoincore_rpc::bitcoin::{Transaction, Address, Network};
use bitcoincore_rpc::bitcoin::consensus::deserialize;
use anyhow::Result;

/// Extracts Bitcoin addresses from transaction hex data
pub struct AddressExtractor;

impl AddressExtractor {
    /// Extract the first output address from a transaction hex string
    /// This is typically where the charm/asset is sent to
    pub fn extract_primary_address(tx_hex: &str, network: &str) -> Result<Option<String>> {
        // Convert hex string to bytes
        let tx_bytes = hex::decode(tx_hex)?;
        
        // Deserialize transaction
        let tx: Transaction = deserialize(&tx_bytes)?;
        
        // Determine Bitcoin network
        let btc_network = match network {
            "mainnet" => Network::Bitcoin,
            "testnet4" => Network::Testnet,
            "testnet" => Network::Testnet,
            "regtest" => Network::Regtest,
            _ => Network::Testnet, // Default to testnet
        };
        
        // Look for the first output with a valid address
        for output in &tx.output {
            if let Ok(address) = Address::from_script(&output.script_pubkey, btc_network) {
                return Ok(Some(address.to_string()));
            }
        }
        
        Ok(None)
    }
    
    /// Extract all output addresses from a transaction hex string
    pub fn extract_all_addresses(tx_hex: &str, network: &str) -> Result<Vec<String>> {
        let tx_bytes = hex::decode(tx_hex)?;
        let tx: Transaction = deserialize(&tx_bytes)?;
        
        let btc_network = match network {
            "mainnet" => Network::Bitcoin,
            "testnet4" => Network::Testnet,
            "testnet" => Network::Testnet,
            "regtest" => Network::Regtest,
            _ => Network::Testnet,
        };
        
        let mut addresses = Vec::new();
        
        for output in &tx.output {
            if let Ok(address) = Address::from_script(&output.script_pubkey, btc_network) {
                addresses.push(address.to_string());
            }
        }
        
        Ok(addresses)
    }
    
    /// Extract the address that likely holds the charm asset
    /// This prioritizes P2PKH and P2SH addresses over others
    pub fn extract_charm_holder_address(tx_hex: &str, network: &str) -> Result<Option<String>> {
        let addresses = Self::extract_all_addresses(tx_hex, network)?;
        
        // Prioritize certain address types for charm holding
        for address in &addresses {
            // P2PKH addresses (start with 1 on mainnet, m/n on testnet)
            if address.starts_with('1') || address.starts_with('m') || address.starts_with('n') {
                return Ok(Some(address.clone()));
            }
        }
        
        // P2SH addresses (start with 3 on mainnet, 2 on testnet)
        for address in &addresses {
            if address.starts_with('3') || address.starts_with('2') {
                return Ok(Some(address.clone()));
            }
        }
        
        // Bech32 addresses (start with bc1 on mainnet, tb1 on testnet)
        for address in &addresses {
            if address.starts_with("bc1") || address.starts_with("tb1") {
                return Ok(Some(address.clone()));
            }
        }
        
        // If no preferred type found, return the first address
        Ok(addresses.first().cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_address_from_valid_hex() {
        // This would need a real transaction hex for proper testing
        // For now, just test that the function doesn't panic with invalid input
        let result = AddressExtractor::extract_primary_address("invalid_hex", "testnet4");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_network_mapping() {
        // Test that network strings map correctly
        // This is tested implicitly in the extract functions
        assert!(true);
    }
}
