use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create spells table
        manager
            .create_table(
                Table::create()
                    .table(Spells::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Spells::Txid)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Spells::BlockHeight).integer().not_null())
                    .col(
                        ColumnDef::new(Spells::Data)
                            .json_binary()
                            .not_null()
                            .default("'{}'::jsonb"),
                    )
                    .col(
                        ColumnDef::new(Spells::DateCreated)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Spells::AssetType)
                            .string()
                            .not_null()
                            .default("spell"),
                    )
                    .col(
                        ColumnDef::new(Spells::Blockchain)
                            .string()
                            .not_null()
                            .default("Bitcoin"),
                    )
                    .col(
                        ColumnDef::new(Spells::Network)
                            .string()
                            .not_null()
                            .default("testnet4"),
                    )
                    .col(ColumnDef::new(Spells::Address).string())
                    .to_owned(),
            )
            .await?;

        // Create indexes for efficient queries
        manager
            .create_index(
                Index::create()
                    .name("idx_spells_block_height")
                    .table(Spells::Table)
                    .col(Spells::BlockHeight)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_spells_network")
                    .table(Spells::Table)
                    .col(Spells::Network)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_spells_network_block_height")
                    .table(Spells::Table)
                    .col(Spells::Network)
                    .col(Spells::BlockHeight)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes
        manager
            .drop_index(
                Index::drop()
                    .name("idx_spells_network_block_height")
                    .table(Spells::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_spells_network")
                    .table(Spells::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_spells_block_height")
                    .table(Spells::Table)
                    .to_owned(),
            )
            .await?;

        // Drop table
        manager
            .drop_table(Table::drop().table(Spells::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Spells {
    Table,
    Txid,
    BlockHeight,
    Data,
    DateCreated,
    AssetType,
    Blockchain,
    Network,
    Address,
}
