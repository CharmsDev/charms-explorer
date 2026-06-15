//! BTC auto-seeder worker loop. See `seeder/mod.rs` for the rationale.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;
use tokio::time;
use tokio_util::sync::CancellationToken;

use crate::infrastructure::maestro::{
    MaestroAddressTx, MaestroChainTip, MaestroClient, MaestroError, MaestroUtxo,
};
use crate::infrastructure::persistence::repositories::address_transactions_repository::AddressTxInsert;
use crate::infrastructure::persistence::repositories::utxo_repository::UtxoInsert;
use crate::infrastructure::persistence::Repositories;
use crate::utils::logging;

#[derive(Debug, Clone)]
pub struct SeederConfig {
    /// How many unseeded addresses to pull per loop iteration.
    pub batch_size: u64,
    /// Maximum concurrent Maestro requests in-flight.
    pub max_concurrent: usize,
    /// Sleep when the queue is empty (no unseeded addresses).
    pub idle_interval: Duration,
    /// Sleep between batches when there IS work — protects Maestro quota.
    pub batch_interval: Duration,
}

impl Default for SeederConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            max_concurrent: 5,
            idle_interval: Duration::from_secs(30),
            batch_interval: Duration::from_secs(5),
        }
    }
}

pub struct AddressSeeder {
    network: String,
    repos: Repositories,
    maestro: MaestroClient,
    cfg: SeederConfig,
}

impl AddressSeeder {
    pub fn new(
        network: String,
        repos: Repositories,
        maestro: MaestroClient,
        cfg: SeederConfig,
    ) -> Self {
        Self {
            network,
            repos,
            maestro,
            cfg,
        }
    }

    /// Main loop. Honours `cancel` between batches and during sleeps so
    /// shutdown stays under the 30s SHUTDOWN_TIMEOUT in NetworkManager.
    pub async fn run(self, cancel: CancellationToken) {
        logging::log_info(&format!(
            "[{}] 🌱 AddressSeeder started (batch={}, concurrent={})",
            self.network, self.cfg.batch_size, self.cfg.max_concurrent
        ));
        let sem = Arc::new(Semaphore::new(self.cfg.max_concurrent));

        loop {
            if cancel.is_cancelled() {
                logging::log_info(&format!(
                    "[{}] 🛑 AddressSeeder stopping (cancellation requested)",
                    self.network
                ));
                return;
            }

            let batch = match self
                .repos
                .monitored_addresses
                .fetch_unseeded(&self.network, self.cfg.batch_size)
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    logging::log_warning(&format!(
                        "[{}] AddressSeeder fetch_unseeded failed: {}",
                        self.network, e
                    ));
                    sleep_cancellable(&cancel, self.cfg.batch_interval).await;
                    continue;
                }
            };

            if batch.is_empty() {
                sleep_cancellable(&cancel, self.cfg.idle_interval).await;
                continue;
            }

            let mut handles = Vec::with_capacity(batch.len());
            for address in batch {
                let permit = match Arc::clone(&sem).acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => break, // semaphore closed → shutdown
                };
                let network = self.network.clone();
                let repos = self.repos.clone();
                let maestro = self.maestro.clone();
                handles.push(tokio::spawn(async move {
                    let _permit = permit;
                    let result = seed_one(&maestro, &repos, &address, &network).await;
                    if let Err(e) = result {
                        logging::log_warning(&format!(
                            "[{}] AddressSeeder failed for {}: {}",
                            network, address, e
                        ));
                    }
                }));
            }
            for h in handles {
                let _ = h.await;
            }

            sleep_cancellable(&cancel, self.cfg.batch_interval).await;
        }
    }
}

/// Outcome of seeding a single address, used both by the worker loop and
/// the standalone `seed_holders` binary.
#[derive(Debug)]
pub enum SeedError {
    LockBusy,
    Maestro(MaestroError),
    Db(String),
}

impl std::fmt::Display for SeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeedError::LockBusy => write!(f, "advisory lock held by another worker"),
            SeedError::Maestro(e) => write!(f, "maestro: {}", e),
            SeedError::Db(e) => write!(f, "db: {}", e),
        }
    }
}

/// Seed one address end-to-end: lock → fetch from Maestro → persist → mark.
/// Exposed (not just an internal closure) so the backfill binary can reuse
/// the exact same path the worker uses.
pub async fn seed_one(
    maestro: &MaestroClient,
    repos: &Repositories,
    address: &str,
    network: &str,
) -> Result<SeedOutcome, SeedError> {
    let locked = repos
        .monitored_addresses
        .try_advisory_lock(address, network)
        .await
        .map_err(|e| SeedError::Db(e.to_string()))?;
    if !locked {
        return Err(SeedError::LockBusy);
    }

    let result = seed_one_locked(maestro, repos, address, network).await;

    let _ = repos
        .monitored_addresses
        .release_advisory_lock(address, network)
        .await;

    result
}

#[derive(Debug)]
pub struct SeedOutcome {
    pub utxos: usize,
    pub txs: usize,
    pub tip_height: u64,
    pub tip_hash: String,
}

async fn seed_one_locked(
    maestro: &MaestroClient,
    repos: &Repositories,
    address: &str,
    network: &str,
) -> Result<SeedOutcome, SeedError> {
    let utxos: Vec<MaestroUtxo> = maestro
        .get_utxos(address)
        .await
        .map_err(SeedError::Maestro)?;
    let txs: Vec<MaestroAddressTx> = maestro
        .get_address_txs(address)
        .await
        .map_err(SeedError::Maestro)?;
    let tip: MaestroChainTip = maestro.get_chain_tip().await.map_err(SeedError::Maestro)?;

    let utxo_inserts: Vec<UtxoInsert> = utxos
        .iter()
        .map(|u| UtxoInsert {
            txid: u.txid.clone(),
            vout: u.vout as i32,
            address: address.to_string(),
            value: u.value as i64,
            script_pubkey: String::new(),
            // 0 = mempool marker for unconfirmed; concrete height for confirmed.
            block_height: u.block_height.unwrap_or(0),
            network: network.to_string(),
            source: "maestro".to_string(),
        })
        .collect();
    let utxo_count = utxo_inserts.len();
    if !utxo_inserts.is_empty() {
        repos
            .utxo
            .insert_batch(&utxo_inserts)
            .await
            .map_err(|e| SeedError::Db(e.to_string()))?;
    }

    let tx_inserts: Vec<AddressTxInsert> = txs
        .iter()
        .map(|t| AddressTxInsert {
            txid: t.txid.clone(),
            address: address.to_string(),
            network: network.to_string(),
            direction: t.direction.clone(),
            amount: t.amount,
            fee: t.fee,
            block_height: t.block_height,
            block_time: t.block_time,
            confirmations: if t.confirmed { 1 } else { 0 },
        })
        .collect();
    let tx_count = tx_inserts.len();
    if !tx_inserts.is_empty() {
        repos
            .address_transactions
            .insert_batch(&tx_inserts)
            .await
            .map_err(|e| SeedError::Db(e.to_string()))?;
    }

    repos
        .monitored_addresses
        .mark_seeded(address, network, tip.height as i32, &tip.hash)
        .await
        .map_err(|e| SeedError::Db(e.to_string()))?;

    Ok(SeedOutcome {
        utxos: utxo_count,
        txs: tx_count,
        tip_height: tip.height,
        tip_hash: tip.hash,
    })
}

async fn sleep_cancellable(cancel: &CancellationToken, d: Duration) {
    tokio::select! {
        _ = time::sleep(d) => {}
        _ = cancel.cancelled() => {}
    }
}
