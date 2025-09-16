use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::application::indexer::processor_trait::BlockchainProcessor;
use crate::application::indexer::BitcoinProcessor;
use crate::config::{AppConfig, NetworkId, NetworkType};
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::{BitcoinClient, BitcoinClientError};
use crate::infrastructure::persistence::repositories::charm_repository::CharmRepository;
use crate::infrastructure::persistence::repositories::{BookmarkRepository, SummaryRepository, TransactionRepository};
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

    /// Initialize processors for all enabled networks
    pub async fn initialize(
        &mut self,
        bookmark_repository: BookmarkRepository,
        charm_repository: CharmRepository,
        transaction_repository: TransactionRepository,
        summary_repository: SummaryRepository,
    ) -> Result<(), BlockProcessorError> {
        // Initialize Bitcoin processors
        if self.config.indexer.enable_bitcoin_testnet4 {
            self.initialize_bitcoin_processor(
                "testnet4",
                bookmark_repository.clone(),
                charm_repository.clone(),
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

        let bitcoin_client = match BitcoinClient::new(bitcoin_config) {
            Ok(client) => client,
            Err(e) => {
                let error_details = match &e {
                    BitcoinClientError::RpcError(rpc_err) => {
                        format!("Bitcoin RPC error: {}", rpc_err)
                    }
                    BitcoinClientError::ConnectionError(conn_err) => {
                        format!("Connection error: {}", conn_err)
                    }
                    _ => format!("{}", e),
                };

                logging::log_error(&format!(
                    "Failed to create Bitcoin client for network '{}': {}",
                    network, error_details
                ));
                return Err(BlockProcessorError::BitcoinClientError(e));
            }
        };

        let api_client = match crate::infrastructure::api::ApiClient::new(&self.config) {
            Ok(client) => client,
            Err(e) => {
                logging::log_error(&format!("Failed to create API client: {}", e));
                return Err(BlockProcessorError::ApiClientError(e));
            }
        };

        let charm_service = CharmService::new(bitcoin_client.clone(), api_client, charm_repository);

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
