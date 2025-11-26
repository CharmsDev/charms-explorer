///! Charm detection logic using native charms-client library

use chrono::Utc;
use serde_json::json;

use crate::domain::errors::CharmError;
use crate::domain::models::Charm;
use crate::domain::services::native_charm_parser::NativeCharmParser;
use crate::domain::services::address_extractor::AddressExtractor;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::repositories::CharmRepository;
use crate::domain::services::charm_queue_service::CharmQueueService;

/// Handles charm detection from Bitcoin transactions
pub struct CharmDetector<'a> {
    bitcoin_client: &'a BitcoinClient,
    charm_repository: &'a CharmRepository,
    charm_queue_service: &'a Option<CharmQueueService>,
}

impl<'a> CharmDetector<'a> {
    pub fn new(
        bitcoin_client: &'a BitcoinClient,
        charm_repository: &'a CharmRepository,
        charm_queue_service: &'a Option<CharmQueueService>,
    ) -> Self {
        Self {
            bitcoin_client,
            charm_repository,
            charm_queue_service,
        }
    }

    /// Detects and processes a potential charm transaction using native parsing
    pub async fn detect_and_process_charm(
        &self,
        txid: &str,
        block_height: u64,
        block_hash: Option<&bitcoincore_rpc::bitcoin::BlockHash>,
        tx_pos: usize,
    ) -> Result<Option<Charm>, CharmError> {
        // Get raw transaction with block hash if provided
        let raw_tx_hex = self
            .bitcoin_client
            .get_raw_transaction_hex(txid, block_hash)
            .await?;

        self.detect_from_hex(txid, block_height, &raw_tx_hex, tx_pos, block_height).await
    }

    /// Detects and processes a potential charm transaction from pre-fetched raw hex
    pub async fn detect_from_hex(
        &self,
        txid: &str,
        block_height: u64,
        raw_tx_hex: &str,
        tx_pos: usize,
        latest_height: u64,
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

        // Try to extract and verify spell using native parser
        let (normalized_spell_opt, charm_json) = match NativeCharmParser::extract_and_verify_charm(raw_tx_hex, false) {
            Ok(spell) => {
                let charm_json = serde_json::to_value(&spell)
                    .map_err(|e| CharmError::ProcessingError(format!("Failed to serialize spell data: {}", e)))?;
                
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

        // Extract asset information from the spell
        let asset_infos = if let Some(ref spell) = normalized_spell_opt {
            NativeCharmParser::extract_asset_info(spell)
        } else {
            vec![]
        };

        // Extract Bitcoin address from the transaction
        let address = match AddressExtractor::extract_charm_holder_address(&raw_tx_hex, &network) {
            Ok(addr) => addr,
            Err(e) => {
                crate::utils::logging::log_debug(&format!(
                    "[{}] ⚠️ Block {}: Could not extract address for tx {}: {}",
                    network, block_height, txid, e
                ));
                None
            }
        };

        // Create charm with appropriate data
        // [RJJ-S01] Removed charmid parameter, added app_id and amount
        let charm = Charm::new(
            txid.to_string(),
            0, // vout - Charms are always in output 0 per protocol
            block_height,
            charm_json,
            Utc::now().naive_utc(),
            "spell".to_string(),
            blockchain.clone(),
            network.clone(),
            address,
            false, // New charms are unspent by default
            String::from("other"), // Default app_id for old detection method
            0, // Default amount for old detection method
        );

        // Extract assets from the spell if available
        let asset_requests = if let Some(ref spell) = normalized_spell_opt {
            let assets = NativeCharmParser::extract_asset_info(spell);
            assets.into_iter().map(|asset| {
                use crate::infrastructure::queue::charm_queue::AssetSaveRequest;
                AssetSaveRequest {
                    app_id: asset.app_id,
                    asset_type: asset.asset_type,
                    supply: asset.amount,
                    blockchain: blockchain.clone(),
                    network: network.clone(),
                }
            }).collect()
        } else {
            vec![]
        };

        // Save the charm to the database (using queue if available, otherwise direct save)
        if let Some(ref queue_service) = self.charm_queue_service {
            match queue_service.save_charm_data(&charm, tx_pos as i64, raw_tx_hex.to_string(), latest_height, asset_requests).await {
                Ok(_) => Ok(Some(charm)),
                Err(e) => {
                    crate::utils::logging::log_error(&format!(
                        "[{}] ❌ Block {}: Failed to queue charm data for tx {} at position {}: {}",
                        network, block_height, txid, tx_pos, e
                    ));
                    Err(CharmError::ProcessingError(format!(
                        "Failed to queue charm data: {}",
                        e
                    )))
                }
            }
        } else {
            crate::utils::logging::log_info(&format!(
                "[{}] 💾 Block {}: Saving charm for tx {} to database (direct)",
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
    }
}
