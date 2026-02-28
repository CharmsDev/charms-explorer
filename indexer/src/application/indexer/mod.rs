//! Production Indexer Module
//!
//! Real-time blockchain indexing for new blocks and mempool.
//! For historical reindexing, see `application::reindexer`.

pub mod block;
pub mod mempool;
pub mod network_manager;
pub mod processor_trait;

pub use block::BitcoinProcessor;
pub use network_manager::NetworkManager;
pub use processor_trait::BlockchainProcessor;
