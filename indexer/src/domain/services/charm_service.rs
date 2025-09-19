use chrono::Utc;
use serde_json::{self, json, Value};
use std::fmt;

use crate::domain::errors::CharmError;
use crate::domain::models::{Asset, Charm};
use crate::domain::services::native_charm_parser::NativeCharmParser;
use crate::domain::services::address_extractor::AddressExtractor;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::repositories::{asset_repository::AssetRepository, charm_repository::CharmRepository};

/// Handles charm detection, processing and storage using native charms-client library
#[derive(Clone)]
pub struct CharmService {
    bitcoin_client: BitcoinClient,
    charm_repository: CharmRepository,
    asset_repository: AssetRepository,
}

impl fmt::Debug for CharmService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CharmService")
            .field("bitcoin_client", &self.bitcoin_client)
            .finish_non_exhaustive()
    }
}

impl CharmService {
    /// Creates a new CharmService with required dependencies
    pub fn new(
        bitcoin_client: BitcoinClient,
        charm_repository: CharmRepository,
        asset_repository: AssetRepository,
    ) -> Self {
        Self {
            bitcoin_client,
            charm_repository,
            asset_repository,
        }
    }

    /// Detects and processes a potential charm transaction using native parsing
    pub async fn detect_and_process_charm(
        &self,
        txid: &str,
        block_height: u64,
        block_hash: Option<&bitcoincore_rpc::bitcoin::BlockHash>,
    ) -> Result<Option<Charm>, CharmError> {
        self.detect_and_process_charm_native(txid, block_height, block_hash, 0).await
    }

    /// Detects and processes a potential charm transaction with context for better logging
    pub async fn detect_and_process_charm_with_context(
        &self,
        txid: &str,
        block_height: u64,
        block_hash: Option<&bitcoincore_rpc::bitcoin::BlockHash>,
        tx_pos: usize,
    ) -> Result<Option<Charm>, CharmError> {
        self.detect_and_process_charm_native(txid, block_height, block_hash, tx_pos).await
    }

    /// Detects and processes a potential charm transaction using native parsing
    /// This method uses the charms-client crate for direct parsing and verification
    pub async fn detect_and_process_charm_native(
        &self,
        txid: &str,
        block_height: u64,
        block_hash: Option<&bitcoincore_rpc::bitcoin::BlockHash>,
        tx_pos: usize,
    ) -> Result<Option<Charm>, CharmError> {
        // Get blockchain and network information from the Bitcoin client
        let blockchain = "Bitcoin".to_string();
        let network = self.bitcoin_client.network_id().name.clone();

        // Log transaction processing start (reduced frequency)
        if tx_pos % 100 == 0 {
            crate::utils::logging::log_info(&format!(
                "[{}] 🔍 Block {}: Processing tx {} (pos: {}) for charm detection",
                network, block_height, txid, tx_pos
            ));
        }

        // Get raw transaction with block hash if provided
        let raw_tx_hex = self
            .bitcoin_client
            .get_raw_transaction_hex(txid, block_hash)
            .await?;

        crate::utils::logging::log_debug(&format!(
            "[{}] 📄 Block {}: Got raw tx hex for {} (length: {} bytes)",
            network, block_height, txid, raw_tx_hex.len()
        ));

        // Try to extract and verify spell using native parser
        let (normalized_spell_opt, charm_json) = match NativeCharmParser::extract_and_verify_charm(&raw_tx_hex, false) {
            Ok(spell) => {
                crate::utils::logging::log_info(&format!(
                    "[{}] ✨ Block {}: CHARM DETECTED in tx {} (pos: {})",
                    network, block_height, txid, tx_pos
                ));
                
                let charm_json = serde_json::to_value(&spell)
                    .map_err(|e| CharmError::ProcessingError(format!("Failed to serialize spell data: {}", e)))?;
                
                crate::utils::logging::log_debug(&format!(
                    "[{}] 📋 Block {}: Charm data for tx {}: {}",
                    network, block_height, txid, serde_json::to_string_pretty(&charm_json).unwrap_or_else(|_| "Failed to serialize".to_string())
                ));
                
                (Some(spell), json!({
                    "type": "spell",
                    "detected": true,
                    "has_native_data": true,
                    "native_data": charm_json,
                    "version": "native_parser"
                }))
            }
            Err(e) => {
                crate::utils::logging::log_debug(&format!(
                    "[{}] ⚪ Block {}: No charm detected in tx {} (pos: {}): {}",
                    network, block_height, txid, tx_pos, e
                ));
                return Ok(None);
            }
        };

        // Extract asset information from the normalized spell if available
        let asset_infos = if let Some(ref spell) = normalized_spell_opt {
            let assets = NativeCharmParser::extract_asset_info(spell);
            crate::utils::logging::log_info(&format!(
                "[{}] 🎯 Block {}: Extracted {} assets from charm tx {}",
                network, block_height, assets.len(), txid
            ));
            assets
        } else {
            vec![]
        };

        // Extract Bitcoin address from the transaction
        let address = match AddressExtractor::extract_charm_holder_address(&raw_tx_hex, &network) {
            Ok(addr) => {
                if let Some(ref addr_str) = addr {
                    crate::utils::logging::log_info(&format!(
                        "[{}] 🏠 Block {}: Extracted address {} for charm tx {}",
                        network, block_height, addr_str, txid
                    ));
                }
                addr
            }
            Err(e) => {
                crate::utils::logging::log_debug(&format!(
                    "[{}] ⚠️ Block {}: Could not extract address for tx {}: {}",
                    network, block_height, txid, e
                ));
                None
            }
        };

        // Create charm with appropriate data
        let charm = Charm::new(
            txid.to_string(),
            format!("charm-{}", txid),
            block_height,
            charm_json,
            Utc::now().naive_utc(),
            "spell".to_string(),
            blockchain.clone(),
            network.clone(),
            address,
        );

        // Always create assets for detected charms
        if !asset_infos.is_empty() {
            for (i, asset_info) in asset_infos.iter().enumerate() {
                crate::utils::logging::log_info(&format!(
                    "[{}] 💎 Block {}: Asset {} - app_id: {}, amount: {}, type: {}, vout: {}",
                    network, block_height, i + 1, asset_info.app_id, asset_info.amount, asset_info.asset_type, asset_info.vout_index
                ));
            }

            for asset_info in &asset_infos {
                let asset = Asset::new(
                    asset_info.app_id.clone(),
                    txid.to_string(),
                    asset_info.vout_index,
                    format!("charm-{}", txid),
                    block_height,
                    Utc::now().naive_utc(),
                    json!({
                        "amount": asset_info.amount,
                        "asset_type": asset_info.asset_type,
                        "extracted_by": "native_parser"
                    }),
                    asset_info.asset_type.clone(),
                    blockchain.clone(),
                    network.clone(),
                );

                // Save individual asset with supply accumulation
                match self.asset_repository.save_or_update_asset(&asset, asset_info.amount as i64).await {
                    Ok(_) => {
                        crate::utils::logging::log_info(&format!(
                            "[{}] ✅ Block {}: Successfully saved asset '{}' for tx {} (amount: {})",
                            network, block_height, asset_info.app_id, txid, asset_info.amount
                        ));
                    }
                    Err(e) => {
                        crate::utils::logging::log_error(&format!(
                            "[{}] ❌ Block {}: Failed to save asset '{}' for tx {} at position {}: {}",
                            network, block_height, asset_info.app_id, txid, tx_pos, e
                        ));
                        // Continue processing even if asset save fails
                    }
                }
            }
        }

        // Save the charm to the database
        crate::utils::logging::log_info(&format!(
            "[{}] 💾 Block {}: Saving charm for tx {} to database",
            network, block_height, txid
        ));
        
        match self.charm_repository.save_charm(&charm).await {
            Ok(_) => {
                crate::utils::logging::log_info(&format!(
                    "[{}] ✅ Block {}: Successfully saved charm for tx {} with {} assets",
                    network, block_height, txid, asset_infos.len()
                ));
                Ok(Some(charm))
            }
            Err(e) => {
                crate::utils::logging::log_error(&format!(
                    "[{}] ❌ Block {}: Failed to save native charm for tx {} at position {}: {}",
                    network, block_height, txid, tx_pos, e
                ));
                Err(CharmError::ProcessingError(format!(
                    "Failed to save native charm: {}",
                    e
                )))
            }
        }
    }

    /// Saves multiple charms in a single database operation
    pub async fn save_batch(
        &self,
        charms: Vec<(
            String,
            String,
            u64,
            serde_json::Value,
            String,
            String,
            String,
        )>,
    ) -> Result<(), CharmError> {
        self.charm_repository
            .save_batch(charms)
            .await
            .map_err(|e| CharmError::ProcessingError(format!("Failed to save charm batch: {}", e)))
    }

    /// Save a batch of assets to the repository
    /// 
    /// Converts simplified asset batch items into the full tuple format expected by the repository
    pub async fn save_asset_batch(
        &self,
        batch: Vec<(
            String, // app_id
            String, // asset_type
            u64,    // supply
            String, // blockchain
            String, // network
        )>,
    ) -> Result<(), CharmError> {
        if batch.is_empty() {
            return Ok(());
        }

        // Transform simplified batch items into full repository format
        let asset_tuples: Vec<(String, String, i32, String, u64, Value, String, String, String)> = batch
            .into_iter()
            .map(|(app_id, asset_type, supply, blockchain, network)| {
                (
                    app_id.clone(),                          // app_id
                    String::new(),                           // txid - empty for asset records
                    0,                                       // vout_index - not applicable for assets
                    format!("charm-{}", app_id),             // charm_id derived from app_id
                    0,                                       // block_height - will be updated during processing
                    serde_json::json!({"supply": supply}),   // data with supply information
                    asset_type,                              // asset_type
                    blockchain,                              // blockchain
                    network,                                 // network
                )
            })
            .collect();

        self.asset_repository
            .save_batch(asset_tuples)
            .await
            .map_err(|e| CharmError::ProcessingError(format!("Failed to save asset batch: {}", e)))
    }
}
