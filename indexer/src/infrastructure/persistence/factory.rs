use sea_orm::DatabaseConnection;

use crate::infrastructure::persistence::connection::DbPool;
use crate::infrastructure::persistence::repositories::{
    AssetRepository, BlockStatusRepository, CharmRepository, DexOrdersRepository,
    MempoolSpendsRepository, MonitoredAddressesRepository, Repositories, SpellRepository,
    StatsHoldersRepository, SummaryRepository, TransactionRepository, UtxoRepository,
};

/// Factory for creating repositories
pub struct RepositoryFactory;

impl RepositoryFactory {
    /// Create all repositories
    /// [RJJ-S01] Now includes spell repository
    /// [RJJ-STATS-HOLDERS] Now includes stats_holders repository
    /// [RJJ-DEX] Now includes dex_orders repository
    pub fn create_repositories(db_pool: &DbPool) -> Repositories {
        let conn = db_pool.get_connection().clone();

        Repositories::new(
            Self::create_asset_repository(conn.clone()),
            Self::create_block_status_repository(conn.clone()),
            Self::create_charm_repository(conn.clone()),
            Self::create_dex_orders_repository(conn.clone()),
            Self::create_spell_repository(conn.clone()),
            Self::create_stats_holders_repository(conn.clone()),
            Self::create_summary_repository(conn.clone()),
            Self::create_transaction_repository(conn.clone()),
            Self::create_utxo_repository(conn.clone()),
            Self::create_monitored_addresses_repository(conn.clone()),
            Self::create_mempool_spends_repository(conn),
        )
    }

    /// Create a charm repository
    pub fn create_charm_repository(conn: DatabaseConnection) -> CharmRepository {
        CharmRepository::new(conn)
    }

    /// Create a summary repository
    pub fn create_summary_repository(conn: DatabaseConnection) -> SummaryRepository {
        SummaryRepository::new(conn)
    }

    /// Create a transaction repository
    pub fn create_transaction_repository(conn: DatabaseConnection) -> TransactionRepository {
        TransactionRepository::new(conn)
    }

    /// Create an asset repository
    pub fn create_asset_repository(conn: DatabaseConnection) -> AssetRepository {
        AssetRepository::new(conn)
    }

    /// Create a spell repository
    /// [RJJ-S01] New repository for spell persistence
    pub fn create_spell_repository(conn: DatabaseConnection) -> SpellRepository {
        SpellRepository::new(conn)
    }

    /// Create a stats_holders repository
    /// [RJJ-STATS-HOLDERS] New repository for holder statistics
    pub fn create_stats_holders_repository(conn: DatabaseConnection) -> StatsHoldersRepository {
        StatsHoldersRepository::new(conn)
    }

    /// Create a dex_orders repository
    /// [RJJ-DEX] New repository for Cast DEX order tracking
    pub fn create_dex_orders_repository(conn: DatabaseConnection) -> DexOrdersRepository {
        DexOrdersRepository::new(conn)
    }

    /// Create a block_status repository
    pub fn create_block_status_repository(conn: DatabaseConnection) -> BlockStatusRepository {
        BlockStatusRepository::new(conn)
    }

    /// Create a utxo repository
    pub fn create_utxo_repository(conn: DatabaseConnection) -> UtxoRepository {
        UtxoRepository::new(conn)
    }

    /// Create a monitored_addresses repository
    pub fn create_monitored_addresses_repository(
        conn: DatabaseConnection,
    ) -> MonitoredAddressesRepository {
        MonitoredAddressesRepository::new(conn)
    }

    /// Create a mempool_spends repository [RJJ-MEMPOOL]
    pub fn create_mempool_spends_repository(conn: DatabaseConnection) -> MempoolSpendsRepository {
        MempoolSpendsRepository::new(conn)
    }
}
