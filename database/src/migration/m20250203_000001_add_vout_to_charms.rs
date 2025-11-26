use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add vout column to charms table
        manager
            .alter_table(
                Table::alter()
                    .table(Charms::Table)
                    .add_column(
                        ColumnDef::new(Charms::Vout)
                            .integer()
                            .not_null()
                            .default(0) // Default to 0 (first output)
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on txid + vout for efficient lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_charms_txid_vout")
                    .table(Charms::Table)
                    .col(Charms::Txid)
                    .col(Charms::Vout)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop index first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_charms_txid_vout")
                    .table(Charms::Table)
                    .to_owned(),
            )
            .await?;

        // Drop vout column
        manager
            .alter_table(
                Table::alter()
                    .table(Charms::Table)
                    .drop_column(Charms::Vout)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Charms {
    Table,
    Txid,
    Vout,
}
