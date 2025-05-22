pub use sea_orm_migration::prelude::*;

mod m20250415_000001_create_tables;
mod m20250415_000002_add_status_fields;
mod m20250508_000001_add_timestamp_to_bookmark;
mod m20250514_000001_add_network_to_bookmark;
mod m20250518_000001_add_blockchain_network_fields;
mod m20250519_000001_add_blockchain_to_bookmark;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250415_000001_create_tables::Migration),
            Box::new(m20250415_000002_add_status_fields::Migration),
            Box::new(m20250508_000001_add_timestamp_to_bookmark::Migration),
            Box::new(m20250514_000001_add_network_to_bookmark::Migration),
            Box::new(m20250518_000001_add_blockchain_network_fields::Migration),
            Box::new(m20250519_000001_add_blockchain_to_bookmark::Migration),
        ]
    }
}
