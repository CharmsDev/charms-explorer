// Database repository management

mod charm_repository;

pub use charm_repository::CharmRepository;

use sea_orm::DatabaseConnection;

/// Container for all database repositories
pub struct Repositories {
    pub charm: CharmRepository,
}

impl Repositories {
    /// Creates a new repositories container with database connection
    pub fn new(conn: DatabaseConnection) -> Self {
        Repositories {
            charm: CharmRepository::new(conn),
        }
    }
}
