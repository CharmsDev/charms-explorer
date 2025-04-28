use reqwest::Client;
use serde_json::Value;

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
        let client = Client::new();
        let api_url = config.api.url.clone();

        Ok(ApiClient { client, api_url })
    }

    /// Get spell data for a transaction
    pub async fn get_spell_data(&self, txid: &str) -> Result<Value, ApiClientError> {
        let url = format!("{}/spells/{}", self.api_url, txid);

        // Make the request
        let response = self.client.get(&url).send().await?;

        // Check if the request was successful
        if response.status().as_u16() == 404 {
            // Not found is a valid response, return an empty object
            println!("API returned 404 for tx: {}", txid);
            return Ok(serde_json::json!({}));
        } else if !response.status().is_success() {
            return Err(ApiClientError::ApiError(format!(
                "API returned error status: {}",
                response.status()
            )));
        }

        // Parse the response as JSON
        match response.json::<Value>().await {
            Ok(json) => Ok(json),
            Err(e) => {
                // If we can't parse the JSON, return an empty object
                if e.to_string().contains("EOF while parsing") {
                    println!("Empty response body for tx: {}", txid);
                    Ok(serde_json::json!({}))
                } else {
                    Err(ApiClientError::ResponseError(format!(
                        "Error decoding response: {}",
                        e
                    )))
                }
            }
        }
    }
}
