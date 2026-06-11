use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::application::indexer::block::BitcoinProcessor;
use crate::application::indexer::mempool::MempoolProcessor;
use crate::application::indexer::processor_trait::BlockchainProcessor;
use crate::application::indexer::supervisor;
use crate::config::{AppConfig, NetworkId, NetworkType};
use crate::domain::errors::BlockProcessorError;
use crate::domain::services::CharmService;
use crate::infrastructure::bitcoin::{BitcoinClient, ProviderFactory, SimpleBitcoinClient};
use crate::infrastructure::persistence::Repositories;
use crate::utils::logging;

/// Manager for multiple blockchain network processors
pub struct NetworkManager {
    config: AppConfig,
    processors: HashMap<String, Arc<Mutex<Box<dyn BlockchainProcessor>>>>,
    tasks: HashMap<String, JoinHandle<Result<(), BlockProcessorError>>>,
    /// Background worker handles (mempool supervisors). Distinct from
    /// `tasks` so block processors and mempool processors can be awaited
    /// separately on shutdown.
    background_tasks: Vec<JoinHandle<()>>,
    /// Fired by `stop_all` to ask every worker to wind down cleanly.
    shutdown: CancellationToken,
}

impl NetworkManager {
    /// Creates a new network manager instance
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            processors: HashMap::new(),
            tasks: HashMap::new(),
            background_tasks: Vec::new(),
            shutdown: CancellationToken::new(),
        }
    }

    /// Initialize processors for all configured networks.
    /// Repos are cloned internally per processor as needed.
    pub async fn initialize(
        &mut self,
        repos: &Repositories,
    ) -> Result<(), BlockProcessorError> {
        if self.config.indexer.enable_bitcoin_testnet4 {
            self.initialize_bitcoin_processor("testnet4", repos).await?;
        }
        if self.config.indexer.enable_bitcoin_mainnet {
            self.initialize_bitcoin_processor("mainnet", repos).await?;
        }
        // TODO: Initialize Cardano processors when implemented
        Ok(())
    }

    /// Initialize a Bitcoin processor for a specific network.
    async fn initialize_bitcoin_processor(
        &mut self,
        network: &str,
        repos: &Repositories,
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

        // Create charm service (synchronous, no queue)
        let charm_service = CharmService::new(
            repos.charm.clone(),
            repos.asset.clone(),
            repos.stats_holders.clone(),
            repos.dex_orders.clone(),
        );

        let processor = BitcoinProcessor::new(
            bitcoin_client,
            charm_service,
            repos.transaction.clone(),
            repos.summary.clone(),
            repos.block_status.clone(),
            repos.utxo.clone(),
            repos.monitored_addresses.clone(),
            repos.mempool_spends.clone(),
            repos.address_transactions.clone(),
            self.config.clone(),
            bitcoin_config.genesis_block_height,
        );

        let network_id = NetworkId::new(NetworkType::Bitcoin, network);
        let network_key = network_id.to_string();
        self.processors.insert(
            network_key.clone(),
            Arc::new(Mutex::new(Box::new(processor))),
        );

        // Spawn the MempoolProcessor under a supervisor so a panic in any
        // poll cycle restarts the worker instead of silently killing it
        // (root cause of the bloque 946,620 incident). The shutdown token
        // lets `stop_all` wind it down cleanly.
        match BitcoinClient::new(bitcoin_config) {
            Ok(mempool_client) => {
                let db_conn = repos.mempool_spends.get_connection();
                let mempool_proc = Arc::new(MempoolProcessor::new(
                    mempool_client,
                    db_conn,
                    repos.mempool_spends.clone(),
                    repos.utxo.clone(),
                    repos.monitored_addresses.clone(),
                    network_id.clone(),
                ));
                let supervisor_name = format!("mempool/{}", network_id.name);
                let cancel = self.shutdown.clone();
                let handle = tokio::spawn(async move {
                    let proc = mempool_proc;
                    supervisor::supervise(&supervisor_name, move || {
                        let proc = proc.clone();
                        let cancel = cancel.clone();
                        async move { proc.run(cancel).await }
                    })
                    .await;
                });
                self.background_tasks.push(handle);
                logging::log_info(&format!(
                    "[{}] 🔍 MempoolProcessor spawned under supervisor",
                    network_id.name
                ));
            }
            Err(e) => {
                logging::log_warning(&format!(
                    "[{}] ⚠️ Could not create mempool RPC client, mempool indexing disabled: {}",
                    network, e
                ));
            }
        }

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

    /// Start a specific processor.
    /// Hands the processor the shared cancellation token so a graceful
    /// shutdown winds it down between block iterations.
    pub async fn start_processor(&mut self, network_key: &str) -> Result<(), BlockProcessorError> {
        if let Some(processor) = self.processors.get(network_key) {
            let processor_clone = processor.clone();
            let cancel = self.shutdown.clone();

            let handle = tokio::spawn(async move {
                let mut processor = processor_clone.lock().await;
                processor.start_processing(cancel).await
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

    /// Graceful shutdown: fire the cancellation token, await both block
    /// processors and background workers (mempool supervisors) until they
    /// wind down or the per-task timeout elapses, then abort anything that
    /// did not honour the cancellation in time.
    pub async fn stop_all(&mut self) {
        const SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

        self.shutdown.cancel();
        logging::log_info("Shutdown signalled, waiting for workers...");

        // Block processors: they observe the token between cycles.
        let tasks = std::mem::take(&mut self.tasks);
        for (name, handle) in tasks {
            match tokio::time::timeout(SHUTDOWN_TIMEOUT, handle).await {
                Ok(Ok(Ok(()))) => {}
                Ok(Ok(Err(e))) => {
                    logging::log_warning(&format!("processor {name} exited with error: {e}"))
                }
                Ok(Err(e)) => logging::log_warning(&format!("processor {name} join error: {e}")),
                Err(_) => {
                    logging::log_warning(&format!("processor {name} did not stop in time; aborting"));
                }
            }
        }

        // Mempool supervisors (already cancellation-aware).
        let background = std::mem::take(&mut self.background_tasks);
        for handle in background {
            match tokio::time::timeout(SHUTDOWN_TIMEOUT, handle).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => logging::log_warning(&format!("background task join error: {e}")),
                Err(_) => logging::log_warning("background task did not stop within timeout"),
            }
        }

        logging::log_info("All processors stopped");
    }
}
