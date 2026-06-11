pub mod address_transactions_repository;
pub mod asset;
pub mod asset_repository;
pub mod block_status_repository;
pub mod charm_repository;
pub mod dex_orders_repository;
pub mod mempool_spends_repository;
pub mod monitored_addresses_repository;
pub mod stats_holders_repository;
pub mod summary_repository;
pub mod transaction_repository;
pub mod utxo_repository;

pub use address_transactions_repository::AddressTransactionsRepository;
pub use asset_repository::AssetRepository;
pub use block_status_repository::BlockStatusRepository;
pub use charm_repository::CharmRepository;
pub use dex_orders_repository::DexOrdersRepository;
pub use mempool_spends_repository::MempoolSpendsRepository;
pub use monitored_addresses_repository::MonitoredAddressesRepository;
pub use stats_holders_repository::StatsHoldersRepository;
pub use summary_repository::SummaryRepository;
pub use transaction_repository::TransactionRepository;
pub use utxo_repository::UtxoRepository;

use crate::infrastructure::persistence::connection::DbPool;

/// Collection of all repositories backed by a shared connection.
#[derive(Clone, Debug)]
pub struct Repositories {
    pub address_transactions: AddressTransactionsRepository,
    pub asset: AssetRepository,
    pub block_status: BlockStatusRepository,
    pub charm: CharmRepository,
    pub dex_orders: DexOrdersRepository,
    pub stats_holders: StatsHoldersRepository,
    pub summary: SummaryRepository,
    pub transaction: TransactionRepository,
    pub utxo: UtxoRepository,
    pub monitored_addresses: MonitoredAddressesRepository,
    pub mempool_spends: MempoolSpendsRepository,
}

impl Repositories {
    pub fn from_pool(db_pool: &DbPool) -> Self {
        let conn = db_pool.get_connection().clone();
        Self {
            address_transactions: AddressTransactionsRepository::new(conn.clone()),
            asset: AssetRepository::new(conn.clone()),
            block_status: BlockStatusRepository::new(conn.clone()),
            charm: CharmRepository::new(conn.clone()),
            dex_orders: DexOrdersRepository::new(conn.clone()),
            stats_holders: StatsHoldersRepository::new(conn.clone()),
            summary: SummaryRepository::new(conn.clone()),
            transaction: TransactionRepository::new(conn.clone()),
            utxo: UtxoRepository::new(conn.clone()),
            monitored_addresses: MonitoredAddressesRepository::new(conn.clone()),
            mempool_spends: MempoolSpendsRepository::new(conn),
        }
    }
}
