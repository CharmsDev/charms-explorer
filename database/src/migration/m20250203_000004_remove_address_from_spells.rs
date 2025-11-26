use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // [RJJ-S01] Remove address column from spells table
        // Spells are OP_RETURN outputs and don't have addresses
        // Only charms (actual UTXOs) have addresses
        manager
            .alter_table(
                Table::alter()
                    .table(Spells::Table)
                    .drop_column(Spells::Address)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add address column back
        manager
            .alter_table(
                Table::alter()
                    .table(Spells::Table)
                    .add_column(
                        ColumnDef::new(Spells::Address)
                            .string()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Spells {
    Table,
    Address,
}
