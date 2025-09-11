// Simplified status handler module exports

pub mod handler;
pub mod network_status;

// Re-export the main handler function
pub use handler::get_indexer_status;
