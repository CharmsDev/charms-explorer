use chrono::Utc;
use serde_json::json;
use std::fmt;

use crate::domain::errors::CharmError;
use crate::domain::models::Charm;
use crate::domain::services::charm_detector::CharmDetectorService;
use crate::infrastructure::api::client::ApiClient;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::repositories::charm_repository::CharmRepository;

/// Handles charm detection, processing and storage
pub struct CharmService {
    bitcoin_client: BitcoinClient,
    api_client: ApiClient,
    charm_repository: CharmRepository,
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
    ) -> Self {
        Self {
            bitcoin_client,
            api_client,
            charm_repository,
        }
    }

    /// Detects and processes a potential charm transaction
    pub async fn detect_and_process_charm(
        &self,
        txid: &str,
        block_height: u64,
        block_hash: Option<&bitcoincore_rpc::bitcoin::BlockHash>,
    ) -> Result<Option<Charm>, CharmError> {
        // Get blockchain and network information from the Bitcoin client
        let blockchain = "Bitcoin".to_string();
        let network = self.bitcoin_client.network_id().name.clone();

        // Get raw transaction with block hash if provided
        let raw_tx_hex = self
            .bitcoin_client
            .get_raw_transaction_hex(txid, block_hash)?;

        // Check if transaction could be a charm
        if !CharmDetectorService::could_be_charm(&raw_tx_hex) {
            return Ok(None);
        }

        // Perform more detailed analysis
        if CharmDetectorService::analyze_charm_transaction(&raw_tx_hex).is_none() {
            return Ok(None);
        }

        // This transaction has potential charm markers, so we should store it
        // regardless of whether the API returns charm data or not

        // Try to fetch spell data from the API
        let (api_data, has_api_data) = match self.api_client.get_spell_data(txid).await {
            Ok(spell_data) => {
                // Check if we got actual data or just an empty response
                if spell_data.is_null()
                    || (spell_data.is_object() && spell_data.as_object().unwrap().is_empty())
                {
                    (json!({"note": "No charm data from API"}), false)
                } else {
                    // Process the spell data
                    let processed_data = CharmDetectorService::process_spell_data(spell_data);
                    (processed_data, true)
                }
            }
            Err(e) => {
                // API failed, but we still want to store this as a potential charm
                crate::utils::logging::log_error(&format!(
                    "❌ API call failed for tx {}: {}",
                    txid, e
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

        // Save the charm to the repository
        self.charm_repository.save_charm(&charm).await?;

        Ok(Some(charm))
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
}
