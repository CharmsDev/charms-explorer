pub mod bookmark_repository;
pub mod charm_repository;
pub mod summary_repository;
pub mod transaction_repository;

pub use bookmark_repository::BookmarkRepository;
pub use charm_repository::CharmRepository;
pub use summary_repository::SummaryRepository;
pub use transaction_repository::TransactionRepository;

/// Collection of all repositories
pub struct Repositories {
    /// Repository for bookmark operations
    pub bookmark: BookmarkRepository,
    /// Repository for charm operations
    pub charm: CharmRepository,
    /// Repository for summary operations
    pub summary: SummaryRepository,
    /// Repository for transaction operations
    pub transaction: TransactionRepository,
}

impl Repositories {
    /// Create a new Repositories instance
    pub fn new(
        bookmark: BookmarkRepository,
        charm: CharmRepository,
        summary: SummaryRepository,
        transaction: TransactionRepository,
    ) -> Self {
        Self {
            bookmark,
            charm,
            summary,
            transaction,
        }
    }
}
