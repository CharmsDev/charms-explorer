// API endpoint handlers implementation

mod assets;
mod charms;
mod dex_orders; // [RJJ-DEX]
mod diagnostic;
mod health;
mod reset;
mod stats_holders; // [RJJ-STATS-HOLDERS]
pub mod status;
pub mod wallet; // [RJJ-WALLET]

use bitcoincore_rpc::Client;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::config::ApiConfig;
use crate::db::Repositories;

// Handler function re-exports
pub use assets::{get_asset_by_id, get_asset_counts, get_assets, get_reference_nft_by_hash};
pub use charms::{
    get_charm_by_charmid, get_charm_by_txid, get_charm_numbers, get_charms, get_charms_by_address,
    get_charms_by_type, get_charms_count_by_type, like_charm, unlike_charm,
};
pub use dex_orders::{get_open_orders, get_order_by_id, get_orders_by_asset, get_orders_by_maker}; // [RJJ-DEX]
pub use diagnostic::diagnose_database;
pub use health::health_check;
pub use stats_holders::get_asset_holders; // [RJJ-STATS-HOLDERS]
pub use status::get_indexer_status;
pub use wallet::{
    broadcast_wallet_transaction, get_wallet_balance, get_wallet_chain_tip,
    get_wallet_charm_balances, get_wallet_fee_estimate, get_wallet_transaction, get_wallet_utxos,
}; // [RJJ-WALLET]

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
}
