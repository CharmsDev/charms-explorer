use sea_orm_migration::prelude::*;

/// [RJJ-DECIMALS] Migration to add decimals field to assets table
/// 
/// This field stores the number of decimal places for token amounts.
/// Default is 8 (Bitcoin standard).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add decimals column to assets table (SMALLINT, default 8)
        // [RJJ-DECIMALS] Range: 0-18 for safety
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .add_column(
                        ColumnDef::new(Assets::Decimals)
                            .small_integer()
                            .not_null()
                            .default(8) // Bitcoin standard
                    )
                    .to_owned(),
            )
            .await?;

        // Add index on decimals for queries
        manager
            .create_index(
                Index::create()
                    .name("idx_assets_decimals")
                    .table(Assets::Table)
                    .col(Assets::Decimals)
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
                    .name("idx_assets_decimals")
                    .table(Assets::Table)
                    .to_owned(),
            )
            .await?;

        // Drop decimals column
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .drop_column(Assets::Decimals)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Assets {
    Table,
    Decimals,
}
