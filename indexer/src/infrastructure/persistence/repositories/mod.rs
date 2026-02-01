pub mod asset;
pub mod asset_repository;
pub mod block_status_repository;
pub mod bookmark_repository;
pub mod charm_repository;
pub mod dex_orders_repository;
pub mod spell_repository;
pub mod stats_holders_repository; // [RJJ-STATS-HOLDERS]
pub mod summary_repository;
pub mod transaction_repository; // [RJJ-DEX]

pub use asset_repository::AssetRepository;
pub use block_status_repository::BlockStatusRepository;
pub use bookmark_repository::BookmarkRepository;
pub use charm_repository::CharmRepository;
pub use dex_orders_repository::DexOrdersRepository;
pub use spell_repository::SpellRepository;
pub use stats_holders_repository::StatsHoldersRepository;
pub use summary_repository::SummaryRepository;
pub use transaction_repository::TransactionRepository; // [RJJ-DEX]

/// Collection of all repositories
/// [RJJ-S01] Added spell repository
/// [RJJ-STATS-HOLDERS] Added stats_holders repository
/// [RJJ-DEX] Added dex_orders repository
pub struct Repositories {
    /// Repository for asset operations
    pub asset: AssetRepository,
    /// Repository for block status tracking
    pub block_status: BlockStatusRepository,
    /// Repository for bookmark operations
    pub bookmark: BookmarkRepository,
    /// Repository for charm operations
    pub charm: CharmRepository,
    /// Repository for DEX orders [RJJ-DEX]
    pub dex_orders: DexOrdersRepository,
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
    /// [RJJ-DEX] Now includes dex_orders repository
    pub fn new(
        asset: AssetRepository,
        block_status: BlockStatusRepository,
        bookmark: BookmarkRepository,
        charm: CharmRepository,
        dex_orders: DexOrdersRepository,       // [RJJ-DEX]
        spell: SpellRepository,                // [RJJ-S01]
        stats_holders: StatsHoldersRepository, // [RJJ-STATS-HOLDERS]
        summary: SummaryRepository,
        transaction: TransactionRepository,
    ) -> Self {
        Self {
            asset,
            block_status,
            bookmark,
            charm,
            dex_orders,    // [RJJ-DEX]
            spell,         // [RJJ-S01]
            stats_holders, // [RJJ-STATS-HOLDERS]
            summary,
            transaction,
        }
    }
}
