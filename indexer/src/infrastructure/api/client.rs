use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use crate::config::AppConfig;
use crate::infrastructure::api::error::ApiClientError;

/// Client for interacting with the Charms API
pub struct ApiClient {
    client: Client,
    api_url: String,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(config: &AppConfig) -> Result<Self, ApiClientError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10)) // 10 second timeout
            .connect_timeout(Duration::from_secs(5)) // 5 second connection timeout
            .build()
            .map_err(|e| {
                ApiClientError::ResponseError(format!("Failed to create HTTP client: {}", e))
            })?;
        let api_url = config.api.url.clone();

        Ok(ApiClient { client, api_url })
    }

    /// Get spell data for a transaction
    pub async fn get_spell_data(&self, txid: &str) -> Result<Value, ApiClientError> {
        let url = format!("{}/spells/{}", self.api_url, txid);

        // Make the request
        let response = self.client.get(&url).send().await?;
        let status = response.status();

        // Check if the request was successful
        if status.as_u16() == 404 {
            // Not found is a valid response, return an empty object
            return Ok(serde_json::json!({}));
        } else if status.as_u16() == 400 {
            // Bad request - likely not a valid charm transaction
            return Ok(serde_json::json!({}));
        } else if !status.is_success() {
            // Only log errors, not successful requests
            eprintln!("API returned error status {} for tx: {}", status, txid);
            return Err(ApiClientError::ApiError(format!(
                "API returned error status: {}",
                status
            )));
        }

        // Parse the response as JSON
        match response.json::<Value>().await {
            Ok(json) => {
                // Only log if we actually found charm data
                if !json.is_null() && !(json.is_object() && json.as_object().unwrap().is_empty()) {
                    println!("✅ Found charm data for tx: {}", txid);
                }
                Ok(json)
            }
            Err(e) => {
                // If we can't parse the JSON, return an empty object
                if e.to_string().contains("EOF while parsing") {
                    Ok(serde_json::json!({}))
                } else {
                    eprintln!("Error decoding response for tx {}: {}", txid, e);
                    Err(ApiClientError::ResponseError(format!(
                        "Error decoding response: {}",
                        e
                    )))
                }
            }
        }
    }
}
