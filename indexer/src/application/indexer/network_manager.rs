use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::application::indexer::BitcoinProcessor;
use crate::application::indexer::processor_trait::BlockchainProcessor;
use crate::config::{AppConfig, NetworkId, NetworkType};
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::{BitcoinClient, SimpleBitcoinClient, ProviderFactory};
use crate::infrastructure::persistence::repositories::{
    AssetRepository, BookmarkRepository, CharmRepository, SummaryRepository, TransactionRepository,
};
use crate::utils::logging;

/// Manager for multiple blockchain network processors
pub struct NetworkManager {
    config: AppConfig,
    processors: HashMap<String, Arc<Mutex<Box<dyn BlockchainProcessor>>>>,
    tasks: HashMap<String, JoinHandle<Result<(), BlockProcessorError>>>,
}

impl NetworkManager {
    /// Creates a new network manager instance
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            processors: HashMap::new(),
            tasks: HashMap::new(),
        }
    }

    /// Initialize processors for all configured networks
    ///
    /// Creates and configures blockchain processors for each network defined in the configuration
    pub async fn initialize(
        &mut self,
        charm_repository: CharmRepository,
        asset_repository: AssetRepository,
        transaction_repository: TransactionRepository,
        bookmark_repository: BookmarkRepository,
        summary_repository: SummaryRepository,
    ) -> Result<(), BlockProcessorError> {
        // Initialize Bitcoin processors
        if self.config.indexer.enable_bitcoin_testnet4 {
            self.initialize_bitcoin_processor(
                "testnet4",
                bookmark_repository.clone(),
                charm_repository.clone(),
                asset_repository.clone(),
                transaction_repository.clone(),
                summary_repository.clone(),
            )
            .await?;
        }


        if self.config.indexer.enable_bitcoin_mainnet {
            self.initialize_bitcoin_processor(
                "mainnet",
                bookmark_repository.clone(),
                charm_repository.clone(),
                asset_repository.clone(),
                transaction_repository.clone(),
                summary_repository.clone(),
            )
            .await?;
        }

        // TODO: Initialize Cardano processors when implemented

        Ok(())
    }

    /// Initialize a Bitcoin processor for a specific network
    async fn initialize_bitcoin_processor(
        &mut self,
        network: &str,
        bookmark_repository: BookmarkRepository,
        charm_repository: CharmRepository,
        asset_repository: AssetRepository,
        transaction_repository: TransactionRepository,
        summary_repository: SummaryRepository,
    ) -> Result<(), BlockProcessorError> {
        let bitcoin_config = match self.config.get_bitcoin_config(network) {
            Some(config) => config,
            None => {
                logging::log_error(&format!(
                    "Bitcoin configuration for network '{}' not found",
                    network
                ));
                return Err(BlockProcessorError::ConfigError(format!(
                    "Bitcoin configuration for network '{}' not found",
                    network
                )));
            }
        };

        // Create network ID
        let network_id = NetworkId::new(NetworkType::Bitcoin, network);

        // Create SimpleBitcoinClient using the new provider system
        let simple_client = match SimpleBitcoinClient::new(bitcoin_config) {
            Ok(client) => client,
            Err(e) => {
                logging::log_error(&format!(
                    "Failed to create Bitcoin client for network '{}': {}",
                    network, e
                ));
                return Err(BlockProcessorError::BitcoinClientError(e));
            }
        };

        // Log which provider is being used
        let provider_name = ProviderFactory::get_provider_name(bitcoin_config);
        logging::log_info(&format!(
            "[{}] 🔧 Using {} provider",
            network_id.name, provider_name
        ));

        // Wrap in legacy BitcoinClient interface for compatibility
        let bitcoin_client = BitcoinClient::from_simple_client(simple_client);

        let charm_service =
            CharmService::new(bitcoin_client.clone(), charm_repository, asset_repository);

        let processor = BitcoinProcessor::new(
            bitcoin_client,
            charm_service,
            bookmark_repository,
            transaction_repository,
            summary_repository,
            self.config.clone(),
            bitcoin_config.genesis_block_height,
        );

        let network_id = NetworkId::new(NetworkType::Bitcoin, network);
        let network_key = network_id.to_string();
        self.processors.insert(
            network_key.clone(),
            Arc::new(Mutex::new(Box::new(processor))),
        );

        Ok(())
    }

    /// Start all processors
    pub async fn start_all(&mut self) -> Result<(), BlockProcessorError> {
        // Collect keys first to avoid borrowing issues
        let network_keys: Vec<String> = self.processors.keys().cloned().collect();

        for network_key in network_keys {
            self.start_processor(&network_key).await?;
        }

        Ok(())
    }

    /// Start a specific processor
    pub async fn start_processor(&mut self, network_key: &str) -> Result<(), BlockProcessorError> {
        if let Some(processor) = self.processors.get(network_key) {
            let processor_clone = processor.clone();

            let handle = tokio::spawn(async move {
                let mut processor = processor_clone.lock().await;
                processor.start_processing().await
            });

            self.tasks.insert(network_key.to_string(), handle);

            Ok(())
        } else {
            Err(BlockProcessorError::ConfigError(format!(
                "Processor for network '{}' not found",
                network_key
            )))
        }
    }

    /// Stop all processors
    pub async fn stop_all(&mut self) {
        for (_, handle) in self.tasks.drain() {
            handle.abort();
        }
        logging::log_info("All processors stopped");
    }
}
