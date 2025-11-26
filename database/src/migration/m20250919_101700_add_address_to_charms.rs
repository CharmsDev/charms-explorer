use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add address column to charms table
        manager
            .alter_table(
                Table::alter()
                    .table(Charms::Table)
                    .add_column(
                        ColumnDef::new(Charms::Address)
                            .string()
                            .null() // Allow null initially for existing records
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on address column for efficient queries
        manager
            .create_index(
                Index::create()
                    .name("idx_charms_address")
                    .table(Charms::Table)
                    .col(Charms::Address)
                    .to_owned(),
            )
            .await?;

        // Create composite index for address + network queries
        manager
            .create_index(
                Index::create()
                    .name("idx_charms_address_network")
                    .table(Charms::Table)
                    .col(Charms::Address)
                    .col(Charms::Network)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_charms_address_network")
                    .table(Charms::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_charms_address")
                    .table(Charms::Table)
                    .to_owned(),
            )
            .await?;

        // Drop address column
        manager
            .alter_table(
                Table::alter()
                    .table(Charms::Table)
                    .drop_column(Charms::Address)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Charms {
    Table,
    Address,
    Network,
}
