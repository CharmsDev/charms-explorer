use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add spent column to charms table
        manager
            .alter_table(
                Table::alter()
                    .table(Charms::Table)
                    .add_column(
                        ColumnDef::new(Charms::Spent)
                            .boolean()
                            .not_null()
                            .default(false) // Default to false (unspent)
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on spent column for efficient queries
        manager
            .create_index(
                Index::create()
                    .name("idx_charms_spent")
                    .table(Charms::Table)
                    .col(Charms::Spent)
                    .to_owned(),
            )
            .await?;

        // Create composite index for spent + network queries (useful for filtering unspent charms by network)
        manager
            .create_index(
                Index::create()
                    .name("idx_charms_spent_network")
                    .table(Charms::Table)
                    .col(Charms::Spent)
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
                    .name("idx_charms_spent_network")
                    .table(Charms::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_charms_spent")
                    .table(Charms::Table)
                    .to_owned(),
            )
            .await?;

        // Drop spent column
        manager
            .alter_table(
                Table::alter()
                    .table(Charms::Table)
                    .drop_column(Charms::Spent)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Charms {
    Table,
    Spent,
    Network,
}
