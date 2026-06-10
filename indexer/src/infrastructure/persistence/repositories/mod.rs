pub mod address_transactions_repository;
pub mod asset;
pub mod asset_repository;
pub mod block_status_repository;
pub mod charm_repository;
pub mod dex_orders_repository;
pub mod mempool_spends_repository; // [RJJ-MEMPOOL]
pub mod monitored_addresses_repository;
pub mod stats_holders_repository; // [RJJ-STATS-HOLDERS]
pub mod summary_repository;
pub mod transaction_repository; // [RJJ-DEX]
pub mod utxo_repository;

pub use address_transactions_repository::AddressTransactionsRepository;
pub use asset_repository::AssetRepository;
pub use block_status_repository::BlockStatusRepository;
pub use charm_repository::CharmRepository;
pub use dex_orders_repository::DexOrdersRepository;
pub use mempool_spends_repository::MempoolSpendsRepository; // [RJJ-MEMPOOL]
pub use monitored_addresses_repository::MonitoredAddressesRepository;
pub use stats_holders_repository::StatsHoldersRepository;
pub use summary_repository::SummaryRepository;
pub use transaction_repository::TransactionRepository; // [RJJ-DEX]
pub use utxo_repository::UtxoRepository;

/// Collection of all repositories
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
    pub fn new(
        address_transactions: AddressTransactionsRepository,
        asset: AssetRepository,
        block_status: BlockStatusRepository,
        charm: CharmRepository,
        dex_orders: DexOrdersRepository,
        stats_holders: StatsHoldersRepository,
        summary: SummaryRepository,
        transaction: TransactionRepository,
        utxo: UtxoRepository,
        monitored_addresses: MonitoredAddressesRepository,
        mempool_spends: MempoolSpendsRepository,
    ) -> Self {
        Self {
            address_transactions,
            asset,
            block_status,
            charm,
            dex_orders,
            stats_holders,
            summary,
            transaction,
            utxo,
            monitored_addresses,
            mempool_spends,
        }
    }
}
