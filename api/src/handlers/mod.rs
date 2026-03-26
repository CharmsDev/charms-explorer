// API endpoint handlers implementation

mod assets;
mod charms;
mod dex_orders; // [RJJ-DEX]
mod diagnostic;
mod health;
mod reset;
mod stats_holders; // [RJJ-STATS-HOLDERS]
pub mod status;
mod transactions;
pub mod wallet; // [RJJ-WALLET]

use bitcoincore_rpc::Client;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use tokio::sync::Semaphore;

use crate::config::ApiConfig;
use crate::db::Repositories;

// Handler function re-exports
pub use assets::{get_asset_by_id, get_asset_counts, get_assets, get_reference_nft_by_hash};
pub use charms::{
    get_charm_by_charmid, get_charm_by_txid, get_charm_numbers, get_charms, get_charms_by_address,
    get_charms_by_type, get_charms_count_by_type, like_charm, unlike_charm,
};
pub use dex_orders::{get_all_orders, get_open_orders, get_order_by_id, get_orders_by_asset, get_orders_by_maker}; // [RJJ-DEX]
pub use diagnostic::diagnose_database;
pub use health::health_check;
pub use stats_holders::get_asset_holders; // [RJJ-STATS-HOLDERS]
pub use status::get_indexer_status;
pub use transactions::{get_transaction_by_txid, get_transactions};
pub use wallet::{
    broadcast_wallet_transaction, get_wallet_balance, get_wallet_chain_tip,
    get_wallet_charm_balances, get_wallet_charm_balances_batch,
    get_wallet_charm_balances_batch_indexed, get_wallet_fee_estimate,
    get_wallet_prev_txs, get_wallet_transaction, get_wallet_transactions,
    get_wallet_tx_hex, get_wallet_utxos, get_wallet_utxos_batch,
}; // [RJJ-WALLET]

/// Circuit breaker for Maestro API.
/// If consecutive failures reach the threshold, Maestro is bypassed for COOLDOWN_SECS.
/// This prevents every request from paying the Maestro timeout penalty when it's down.
pub struct MaestroCircuitBreaker {
    /// Unix timestamp (secs) when the circuit was opened (0 = closed/healthy)
    pub open_since: AtomicU64,
    /// Consecutive failure count
    pub failures: AtomicU32,
}

impl MaestroCircuitBreaker {
    pub const FAILURE_THRESHOLD: u32 = 2;
    pub const COOLDOWN_SECS: u64 = 120; // 2 minutes

    pub fn new() -> Self {
        Self {
            open_since: AtomicU64::new(0),
            failures: AtomicU32::new(0),
        }
    }

    /// Returns true if Maestro should be skipped (circuit is open and cooldown not expired)
    pub fn is_open(&self) -> bool {
        let opened = self.open_since.load(Ordering::Relaxed);
        if opened == 0 {
            return false;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if now - opened >= Self::COOLDOWN_SECS {
            // Cooldown expired — close circuit, allow retry
            self.open_since.store(0, Ordering::Relaxed);
            self.failures.store(0, Ordering::Relaxed);
            tracing::info!("Maestro circuit breaker: cooldown expired, retrying Maestro");
            false
        } else {
            true
        }
    }

    /// Record a successful call — reset failures and close circuit
    pub fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
        self.open_since.store(0, Ordering::Relaxed);
    }

    /// Record a failed call — increment failures, open circuit if threshold reached
    pub fn record_failure(&self) {
        let count = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        if count >= Self::FAILURE_THRESHOLD {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            self.open_since.store(now, Ordering::Relaxed);
            tracing::warn!(
                "Maestro circuit breaker OPEN: {} consecutive failures, bypassing for {}s",
                count,
                Self::COOLDOWN_SECS
            );
        }
    }
}

/// Application state containing repositories and configuration
#[derive(Clone)]
pub struct AppState {
    pub repositories: Arc<Repositories>,
    pub config: ApiConfig,
    pub scan_semaphore: Arc<Semaphore>,
    pub quicknode_semaphore: Arc<Semaphore>,
    pub http_client: reqwest::Client,
    pub rpc_mainnet: Arc<Client>,
    pub rpc_testnet4: Arc<Client>,
    pub maestro_cb: Arc<MaestroCircuitBreaker>,
}
