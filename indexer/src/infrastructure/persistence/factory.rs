use sea_orm::DatabaseConnection;

use crate::infrastructure::persistence::connection::DbPool;
use crate::infrastructure::persistence::repositories::{
    AssetRepository, BookmarkRepository, CharmRepository, Repositories, SpellRepository, StatsHoldersRepository, SummaryRepository, TransactionRepository, // [RJJ-S01] Added SpellRepository, [RJJ-STATS-HOLDERS] Added StatsHoldersRepository
};

/// Factory for creating repositories
pub struct RepositoryFactory;

impl RepositoryFactory {
    /// Create all repositories
    /// [RJJ-S01] Now includes spell repository
    /// [RJJ-STATS-HOLDERS] Now includes stats_holders repository
    pub fn create_repositories(db_pool: &DbPool) -> Repositories {
        let conn = db_pool.get_connection().clone();

        Repositories::new(
            Self::create_asset_repository(conn.clone()),
            Self::create_bookmark_repository(conn.clone()),
            Self::create_charm_repository(conn.clone()),
            Self::create_spell_repository(conn.clone()), // [RJJ-S01]
            Self::create_stats_holders_repository(conn.clone()), // [RJJ-STATS-HOLDERS]
            Self::create_summary_repository(conn.clone()),
            Self::create_transaction_repository(conn),
        )
    }

    /// Create a bookmark repository
    pub fn create_bookmark_repository(conn: DatabaseConnection) -> BookmarkRepository {
        BookmarkRepository::new(conn)
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
}
