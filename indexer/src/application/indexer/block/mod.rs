//! Block processing module — modular architecture.
//!
//! Each sub-module has a single responsibility:
//! - `bitcoin_processor`: top-level driver (live loop)
//! - `processor`: slim orchestrator for individual block processing
//! - `detection`: charm detection from transactions using TxAnalyzer
//! - `spent_tracker`: marks charms as spent
//! - `utxo_indexer`: registers addresses and tracks UTXOs
//! - `mempool_consolidator`: promotes mempool entries to confirmed
//! - `batch`: batch persistence for charms, transactions, assets
//! - `summary`: summary statistics updater
//! - `retry`: retry handler with exponential backoff

pub mod batch;
pub mod bitcoin_processor;
pub mod detection;
pub mod mempool_consolidator;
pub mod processor;
pub mod reorg;
pub mod retry;
pub mod spent_tracker;
pub mod summary;
pub mod utxo_indexer;

pub use batch::{AssetBatchItem, BatchProcessor, CharmBatchItem, TransactionBatchItem};
pub use bitcoin_processor::BitcoinProcessor;
pub use processor::BlockProcessor;
pub use retry::RetryHandler;
pub use summary::SummaryUpdater;
