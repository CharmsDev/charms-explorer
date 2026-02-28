// Database repository management

pub mod address_transactions_repository;
pub mod asset_repository;
pub mod charm_repository;
pub mod dex_orders_repository; // [RJJ-DEX]
pub mod likes_repository;
pub mod monitored_addresses_repository;
pub mod stats_holders_repository; // [RJJ-STATS-HOLDERS]
pub mod transaction_repository; // [RJJ-SPELL]
pub mod utxo_repository;

pub use address_transactions_repository::AddressTransactionsRepository;
pub use asset_repository::AssetRepository;
pub use charm_repository::CharmRepository;
pub use dex_orders_repository::DexOrdersRepository; // [RJJ-DEX]
pub use likes_repository::LikesRepository;
pub use monitored_addresses_repository::MonitoredAddressesRepository;
pub use stats_holders_repository::StatsHoldersRepository;
pub use transaction_repository::TransactionRepository; // [RJJ-SPELL]
pub use utxo_repository::UtxoRepository;

use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// Container for all database repositories
pub struct Repositories {
    pub address_transactions: AddressTransactionsRepository,
    pub asset_repository: Arc<AssetRepository>,
    pub charm: CharmRepository,
    pub dex_orders: DexOrdersRepository, // [RJJ-DEX]
    pub likes: LikesRepository,
    pub stats_holders: StatsHoldersRepository, // [RJJ-STATS-HOLDERS]
    pub transactions: TransactionRepository,   // [RJJ-SPELL]
    pub utxo: UtxoRepository,
    pub monitored_addresses: MonitoredAddressesRepository,
}

impl Repositories {
    /// Creates a new repositories container with database connection
    pub fn new(conn: DatabaseConnection) -> Self {
        let db_conn = conn.clone();
        let db_conn2 = conn.clone();
        let db_conn3 = conn.clone();
        let db_conn4 = conn.clone();
        let db_conn5 = conn.clone();
        let db_conn6 = conn.clone();
        let db_conn7 = conn.clone();
        let db_conn8 = conn.clone();
        Repositories {
            address_transactions: AddressTransactionsRepository::new(db_conn8),
            asset_repository: Arc::new(AssetRepository::new(std::sync::Arc::new(conn))),
            charm: CharmRepository::new(db_conn),
            dex_orders: DexOrdersRepository::new(db_conn5), // [RJJ-DEX]
            likes: LikesRepository::new(db_conn2),
            stats_holders: StatsHoldersRepository::new(db_conn3), // [RJJ-STATS-HOLDERS]
            transactions: TransactionRepository::new(db_conn4),   // [RJJ-SPELL]
            utxo: UtxoRepository::new(db_conn6),
            monitored_addresses: MonitoredAddressesRepository::new(db_conn7),
        }
    }
}
