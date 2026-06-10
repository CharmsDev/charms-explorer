use bitcoincore_rpc::bitcoin::{Transaction, Address, Network};
use bitcoincore_rpc::bitcoin::consensus::deserialize;
use anyhow::Result;

/// Extracts Bitcoin addresses from transaction hex data
pub struct AddressExtractor;

impl AddressExtractor {
    /// Extract all output addresses from a transaction hex string
    fn extract_all_addresses(tx_hex: &str, network: &str) -> Result<Vec<String>> {
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

