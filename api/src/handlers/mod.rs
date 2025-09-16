// API endpoint handlers implementation

mod charms;
mod diagnostic;
mod health;
mod reset;
pub mod status;

use std::sync::Arc;

use crate::config::ApiConfig;
use crate::db::Repositories;

// Handler function re-exports
pub use charms::{
    get_charm_by_charmid, get_charm_by_txid, get_charm_numbers, get_charms, get_charms_by_type,
    like_charm, unlike_charm,
};
pub use diagnostic::diagnose_database;
pub use health::health_check;
pub use reset::reset_indexer;
pub use status::get_indexer_status;

/// Application state containing repositories and configuration
#[derive(Clone)]
pub struct AppState {
    pub repositories: Arc<Repositories>,
    pub config: ApiConfig,
}
