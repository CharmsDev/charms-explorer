use chrono::Utc;
use serde_json::{self, json, Value};
use std::fmt;

use crate::domain::errors::CharmError;
use crate::domain::models::{Asset, Charm};
use crate::domain::services::charm_detector::CharmDetectorService;
use crate::infrastructure::api::client::ApiClient;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::repositories::{asset_repository::AssetRepository, charm_repository::CharmRepository};

/// Handles charm detection, processing and storage
pub struct CharmService {
    bitcoin_client: BitcoinClient,
    api_client: ApiClient,
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
        api_client: ApiClient,
        charm_repository: CharmRepository,
        asset_repository: AssetRepository,
    ) -> Self {
        Self {
            bitcoin_client,
            api_client,
            charm_repository,
            asset_repository,
        }
    }

    /// Detects and processes a potential charm transaction
    pub async fn detect_and_process_charm(
        &self,
        txid: &str,
        block_height: u64,
        block_hash: Option<&bitcoincore_rpc::bitcoin::BlockHash>,
    ) -> Result<Option<Charm>, CharmError> {
        self.detect_and_process_charm_with_context(txid, block_height, block_hash, 0).await
    }

    /// Detects and processes a potential charm transaction with context for better logging
    pub async fn detect_and_process_charm_with_context(
        &self,
        txid: &str,
        block_height: u64,
        block_hash: Option<&bitcoincore_rpc::bitcoin::BlockHash>,
        tx_pos: usize,
    ) -> Result<Option<Charm>, CharmError> {
        // Get blockchain and network information from the Bitcoin client
        let blockchain = "Bitcoin".to_string();
        let network = self.bitcoin_client.network_id().name.clone();

        // Get raw transaction with block hash if provided
        let raw_tx_hex = self
            .bitcoin_client
            .get_raw_transaction_hex(txid, block_hash)?;

        // Check if transaction could be a charm with enhanced logging
        if !CharmDetectorService::could_be_charm_with_context(
            &raw_tx_hex,
            txid,
            block_height,
            tx_pos,
            &network,
        ) {
            return Ok(None);
        }

        // Perform more detailed analysis
        if CharmDetectorService::analyze_charm_transaction(&raw_tx_hex).is_none() {
            return Ok(None);
        }

        // This transaction has potential charm markers, so we should store it
        // regardless of whether the API returns charm data or not

        // Log API request with full context
        crate::utils::logging::log_info(&format!(
            "[{}] 🔍 Block {}: Making API request for charm tx {} at position {}",
            network, block_height, txid, tx_pos
        ));

        // Try to fetch spell data from the API
        let (api_data, has_api_data) = match self.api_client.get_spell_data(txid).await {
            Ok(spell_data) => {
                // Check if we got actual data or just an empty response
                if spell_data.is_null()
                    || (spell_data.is_object() && spell_data.as_object().unwrap().is_empty())
                {
                    crate::utils::logging::log_info(&format!(
                        "[{}] ⚪ Block {}: API returned empty response for tx {} at position {}",
                        network, block_height, txid, tx_pos
                    ));
                    (json!({"note": "No charm data from API"}), false)
                } else {
                    crate::utils::logging::log_info(&format!(
                        "[{}] ✅ Block {}: API returned charm data for tx {} at position {} (size: {} bytes)",
                        network, block_height, txid, tx_pos, spell_data.to_string().len()
                    ));
                    // Process the spell data
                    let processed_data = CharmDetectorService::process_spell_data(spell_data);
                    (processed_data, true)
                }
            }
            Err(e) => {
                // API failed, but we still want to store this as a potential charm
                crate::utils::logging::log_error(&format!(
                    "[{}] ❌ Block {}: API call failed for tx {} at position {}: {}",
                    network, block_height, txid, tx_pos, e
                ));
                (
                    json!({"note": "API call failed", "error": e.to_string()}),
                    false,
                )
            }
        };

        // Create charm JSON - always store transactions with charm markers
        let charm_json = json!({
            "type": "spell",
            "detected": true,
            "has_api_data": has_api_data,
            "data": api_data
        });

        // Create a new charm - we store all potential charms
        let charm = Charm::new(
            txid.to_string(),
            format!("charm-{}", txid),
            block_height,
            charm_json.clone(),
            Utc::now().naive_utc(),
            "spell".to_string(),
            blockchain.clone(),
            network.clone(),
        );

        // Extract app_id from API data and create asset if found
        if has_api_data {
            if let Some(app_id) = CharmDetectorService::extract_app_id_from_spell_data(&api_data) {
                crate::utils::logging::log_info(&format!(
                    "[{}] 🎯 Block {}: Extracted app_id '{}' for tx {} at position {}",
                    network, block_height, app_id, txid, tx_pos
                ));

                // Create asset for this app_id
                let asset = Asset::new(
                    app_id.clone(),
                    txid.to_string(),
                    0, // Default vout_index, could be extracted from transaction data
                    format!("charm-{}", txid),
                    block_height,
                    Utc::now().naive_utc(),
                    api_data.clone(),
                    "spell".to_string(),
                    blockchain.clone(),
                    network.clone(),
                );

                // Save the asset to the database
                match self.asset_repository.save_asset(&asset).await {
                    Ok(_) => {
                        crate::utils::logging::log_info(&format!(
                            "[{}] ✅ Block {}: Saved asset with app_id '{}' for tx {} at position {}",
                            network, block_height, app_id, txid, tx_pos
                        ));
                    }
                    Err(e) => {
                        crate::utils::logging::log_error(&format!(
                            "[{}] ❌ Block {}: Failed to save asset with app_id '{}' for tx {} at position {}: {}",
                            network, block_height, app_id, txid, tx_pos, e
                        ));
                        // Continue processing even if asset save fails
                    }
                }
            } else {
                crate::utils::logging::log_info(&format!(
                    "[{}] ⚪ Block {}: No app_id found in API data for tx {} at position {}",
                    network, block_height, txid, tx_pos
                ));
            }
        }

        // Save the charm to the database
        match self.charm_repository.save_charm(&charm).await {
            Ok(_) => {
                crate::utils::logging::log_info(&format!(
                    "[{}] ✅ Block {}: Saved charm for tx {} at position {}",
                    network, block_height, txid, tx_pos
                ));
                Ok(Some(charm))
            }
            Err(e) => {
                crate::utils::logging::log_error(&format!(
                    "[{}] ❌ Block {}: Failed to save charm for tx {} at position {}: {}",
                    network, block_height, txid, tx_pos, e
                ));
                Err(CharmError::ProcessingError(format!(
                    "Failed to save charm: {}",
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
