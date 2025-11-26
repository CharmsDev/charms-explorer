pub mod asset_repository;
pub mod bookmark_repository;
pub mod charm_repository;
pub mod spell_repository;
pub mod stats_holders_repository; // [RJJ-STATS-HOLDERS]
pub mod summary_repository;
pub mod transaction_repository;

pub use asset_repository::AssetRepository;
pub use bookmark_repository::BookmarkRepository;
pub use charm_repository::CharmRepository;
pub use spell_repository::SpellRepository;
pub use stats_holders_repository::StatsHoldersRepository;
pub use summary_repository::SummaryRepository;
pub use transaction_repository::TransactionRepository;

/// Collection of all repositories
/// [RJJ-S01] Added spell repository
/// [RJJ-STATS-HOLDERS] Added stats_holders repository
pub struct Repositories {
    /// Repository for asset operations
    pub asset: AssetRepository,
    /// Repository for bookmark operations
    pub bookmark: BookmarkRepository,
    /// Repository for charm operations
    pub charm: CharmRepository,
    /// Repository for spell operations [RJJ-S01]
    pub spell: SpellRepository,
    /// Repository for holder statistics [RJJ-STATS-HOLDERS]
    pub stats_holders: StatsHoldersRepository,
    /// Repository for summary operations
    pub summary: SummaryRepository,
    /// Repository for transaction operations
    pub transaction: TransactionRepository,
}

impl Repositories {
    /// Create a new Repositories instance
    /// [RJJ-S01] Now includes spell repository
    /// [RJJ-STATS-HOLDERS] Now includes stats_holders repository
    pub fn new(
        asset: AssetRepository,
        bookmark: BookmarkRepository,
        charm: CharmRepository,
        spell: SpellRepository, // [RJJ-S01]
        stats_holders: StatsHoldersRepository, // [RJJ-STATS-HOLDERS]
        summary: SummaryRepository,
        transaction: TransactionRepository,
    ) -> Self {
        Self {
            asset,
            bookmark,
            charm,
            spell, // [RJJ-S01]
            stats_holders, // [RJJ-STATS-HOLDERS]
            summary,
            transaction,
        }
    }
}
