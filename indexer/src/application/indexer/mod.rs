//! Production Indexer Module
//!
//! Real-time blockchain indexing for new blocks and mempool.
//! For historical reindexing, see `application::reindexer`.

pub mod bitcoin_processor;
pub mod mempool_processor; // [RJJ-MEMPOOL]
pub mod network_manager;
pub mod processor_trait;

pub use bitcoin_processor::BitcoinProcessor;
pub use network_manager::NetworkManager;
pub use processor_trait::BlockchainProcessor;
