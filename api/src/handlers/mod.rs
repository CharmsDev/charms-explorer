// API endpoint handlers implementation

mod charms;
mod health;

use std::sync::Arc;

use crate::db::Repositories;

// Handler function re-exports
pub use charms::{get_charm_by_txid, get_charm_numbers, get_charms, get_charms_by_type};
pub use health::health_check;

/// Type alias for the application state (repositories wrapped in Arc)
pub type AppState = Arc<Repositories>;
