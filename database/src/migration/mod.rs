pub use sea_orm_migration::prelude::*;

mod m20250618_000001_create_summary_table;
mod m20250619_000001_create_likes_table;
mod m20250916_000001_create_assets_table;
mod m20250919_101700_add_address_to_charms;
mod m20250202_000001_add_spent_to_charms;
mod m20250203_000001_add_vout_to_charms;
mod m20250203_000002_create_spells_table;
mod m20250203_000003_update_charms_primary_key;
mod m20250203_000004_remove_address_from_spells;
mod m20250203_000005_add_app_id_to_charms;
mod m20250204_000001_add_amount_to_charms;
mod m20250204_000002_add_decimals_to_assets;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250618_000001_create_summary_table::Migration),
            Box::new(m20250619_000001_create_likes_table::Migration),
            Box::new(m20250916_000001_create_assets_table::Migration),
            Box::new(m20250919_101700_add_address_to_charms::Migration),
            Box::new(m20250202_000001_add_spent_to_charms::Migration),
            Box::new(m20250203_000001_add_vout_to_charms::Migration),
            Box::new(m20250203_000002_create_spells_table::Migration),
            Box::new(m20250203_000003_update_charms_primary_key::Migration),
            Box::new(m20250203_000004_remove_address_from_spells::Migration),
            Box::new(m20250203_000005_add_app_id_to_charms::Migration),
            Box::new(m20250204_000001_add_amount_to_charms::Migration),
            Box::new(m20250204_000002_add_decimals_to_assets::Migration),
        ]
    }
}
