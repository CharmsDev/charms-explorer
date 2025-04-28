use chrono::Utc;
use serde_json::json;

use crate::domain::errors::CharmError;
use crate::domain::models::Charm;
use crate::domain::services::charm_detector::CharmDetectorService;
use crate::infrastructure::api::client::ApiClient;
use crate::infrastructure::bitcoin::client::BitcoinClient;
use crate::infrastructure::persistence::repositories::charm_repository::CharmRepository;

/// Service for charm-related operations
pub struct CharmService {
    bitcoin_client: BitcoinClient,
    api_client: ApiClient,
    charm_repository: CharmRepository,
}

impl CharmService {
    /// Create a new CharmService
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

    /// Detect and process a potential charm transaction
    ///
    /// # Arguments
    ///
    /// * `txid` - The transaction ID
    /// * `block_height` - The block height
    ///
    /// # Returns
    ///
    /// Result containing the charm if found, or an error
    pub async fn detect_and_process_charm(
        &self,
        txid: &str,
        block_height: u64,
    ) -> Result<Option<Charm>, CharmError> {
        // Get raw transaction
        let raw_tx_hex = self.bitcoin_client.get_raw_transaction_hex(txid)?;

        // Check if transaction could be a charm
        if !CharmDetectorService::could_be_charm(&raw_tx_hex) {
            return Ok(None);
        }

        // Perform more detailed analysis
        if CharmDetectorService::analyze_charm_transaction(&raw_tx_hex).is_none() {
            return Ok(None);
        }

        // Try to fetch spell data from the API
        match self.api_client.get_spell_data(txid).await {
            Ok(spell_data) => {
                // Process the spell data
                let processed_data = CharmDetectorService::process_spell_data(spell_data);

                // Create charm JSON with the processed spell data
                let charm_json = json!({
                    "type": "spell",
                    "detected": true,
                    "data": processed_data
                });

                // Check if the processed data contains actual charm data or just a "no data" note
                if let Some(data) = processed_data.as_object() {
                    if data.contains_key("note")
                        && data.get("note").unwrap().as_str().unwrap_or("")
                            == "No data available from API"
                    {
                        // This is not a real charm, it's a transaction that was detected as a potential charm
                        // but the API returned no data for it
                        return Ok(None);
                    }
                }

                // Create a new charm
                let charm = Charm::new(
                    txid.to_string(),
                    format!("charm-{}", txid),
                    block_height,
                    charm_json.clone(),
                    Utc::now().naive_utc(),
                    "spell".to_string(),
                );

                // Save the charm to the repository
                self.charm_repository.save_charm(&charm).await?;

                Ok(Some(charm))
            }
            Err(e) => {
                // Failed to fetch spell data
                Err(CharmError::ProcessingError(format!(
                    "Failed to fetch spell data: {}",
                    e
                )))
            }
        }
    }
}
