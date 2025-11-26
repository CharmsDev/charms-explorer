use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add amount column to charms table (BIGINT to store satoshis/units with 8 decimals)
        manager
            .alter_table(
                Table::alter()
                    .table(Charms::Table)
                    .add_column(
                        ColumnDef::new(Charms::Amount)
                            .big_integer()
                            .not_null()
                            .default(0)
                    )
                    .to_owned(),
            )
            .await?;

        // Add index on amount for queries
        manager
            .create_index(
                Index::create()
                    .name("idx_charms_amount")
                    .table(Charms::Table)
                    .col(Charms::Amount)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop index
        manager
            .drop_index(
                Index::drop()
                    .name("idx_charms_amount")
                    .table(Charms::Table)
                    .to_owned(),
            )
            .await?;

        // Drop amount column
        manager
            .alter_table(
                Table::alter()
                    .table(Charms::Table)
                    .drop_column(Charms::Amount)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Charms {
    Table,
    Amount,
}
