// API endpoint handlers implementation

mod assets;
mod charms;
mod diagnostic;
mod health;
mod reset;
mod stats_holders; // [RJJ-STATS-HOLDERS]
pub mod status;

use std::sync::Arc;

use crate::config::ApiConfig;
use crate::db::Repositories;

// Handler function re-exports
pub use assets::{get_assets, get_asset_counts, get_asset_by_id};
pub use charms::{
    get_charm_by_charmid, get_charm_by_txid, get_charm_numbers, get_charms, get_charms_by_address, get_charms_by_type,
    like_charm, unlike_charm,
};
pub use diagnostic::diagnose_database;
pub use health::health_check;
pub use reset::reset_indexer;
pub use stats_holders::get_asset_holders; // [RJJ-STATS-HOLDERS]
pub use status::get_indexer_status;

/// Application state containing repositories and configuration
#[derive(Clone)]
pub struct AppState {
    pub repositories: Arc<Repositories>,
    pub config: ApiConfig,
}
