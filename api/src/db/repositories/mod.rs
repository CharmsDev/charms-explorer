// Database repository management

mod charm_repository;
mod likes_repository;

pub use charm_repository::CharmRepository;
pub use likes_repository::LikesRepository;

use sea_orm::DatabaseConnection;

/// Container for all database repositories
pub struct Repositories {
    pub charm: CharmRepository,
    pub likes: LikesRepository,
}

impl Repositories {
    /// Creates a new repositories container with database connection
    pub fn new(conn: DatabaseConnection) -> Self {
        let db_conn = conn.clone();
        Repositories {
            charm: CharmRepository::new(conn),
            likes: LikesRepository::new(db_conn),
        }
    }
}
