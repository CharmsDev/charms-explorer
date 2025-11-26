//! Provider factory for creating Bitcoin providers based on configuration

use std::sync::Arc;
use crate::config::{BitcoinConfig, ProviderType};
use crate::infrastructure::bitcoin::error::BitcoinClientError;
use crate::infrastructure::bitcoin::providers::{BitcoinProvider, QuickNodeProvider, BitcoinNodeProvider};

/// Factory for creating Bitcoin providers
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a provider based on the configuration
    pub fn create_provider(config: &BitcoinConfig) -> Result<Arc<dyn BitcoinProvider>, BitcoinClientError> {
        match config.provider_type {
            ProviderType::QuickNode => {
                let endpoint = config.quicknode_endpoint
                    .as_ref()
                    .ok_or_else(|| BitcoinClientError::ConfigError(
                        "QuickNode endpoint not configured".to_string()
                    ))?;
                
                let provider = QuickNodeProvider::new(endpoint.clone());
                Ok(Arc::new(provider))
            },
            ProviderType::BitcoinNode => {
                let provider = BitcoinNodeProvider::new(
                    config.host.clone(),
                    config.port.clone(),
                    config.username.clone(),
                    config.password.clone(),
                    config.network.clone(),
                )?;
                Ok(Arc::new(provider))
            }
        }
    }

    /// Get provider name for logging
    pub fn get_provider_name(config: &BitcoinConfig) -> String {
        match config.provider_type {
            ProviderType::QuickNode => "QuickNode".to_string(),
            ProviderType::BitcoinNode => format!("Bitcoin Node ({})", config.network),
        }
    }
}
