use sea_orm::DatabaseConnection;

use crate::infrastructure::persistence::connection::DbPool;
use crate::infrastructure::persistence::repositories::{
    BookmarkRepository, CharmRepository, Repositories, SummaryRepository, TransactionRepository,
};

/// Factory for creating repositories
pub struct RepositoryFactory;

impl RepositoryFactory {
    /// Create all repositories
    pub fn create_repositories(db_pool: &DbPool) -> Repositories {
        let conn = db_pool.get_connection().clone();

        Repositories::new(
            Self::create_bookmark_repository(conn.clone()),
            Self::create_charm_repository(conn.clone()),
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
}
