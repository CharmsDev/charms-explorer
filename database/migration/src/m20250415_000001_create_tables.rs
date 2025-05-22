use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Check if tables already exist
        if !manager.has_table("bookmark").await? {
            // Create bookmark table
            manager
                .create_table(
                    Table::create()
                        .table(Bookmark::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Bookmark::Hash)
                                .string()
                                .not_null()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Bookmark::Height).integer().not_null())
                        .to_owned(),
                )
                .await?;

            // Create index on bookmark height
            // Use if_not_exists to avoid errors if the index already exists
            manager
                .create_index(
                    Index::create()
                        .name("bookmark_height")
                        .table(Bookmark::Table)
                        .col(Bookmark::Height)
                        .if_not_exists()
                        .to_owned(),
                )
                .await?;
        }

        if !manager.has_table("charms").await? {
            // Create charms table
            manager
                .create_table(
                    Table::create()
                        .table(Charms::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Charms::Txid)
                                .string()
                                .not_null()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Charms::Charmid).string().not_null())
                        .col(ColumnDef::new(Charms::BlockHeight).integer().not_null())
                        .col(
                            ColumnDef::new(Charms::Data)
                                .json_binary()
                                .not_null()
                                .default("{}"),
                        )
                        .col(
                            ColumnDef::new(Charms::DateCreated)
                                .timestamp()
                                .not_null()
                                .default(Expr::current_timestamp()),
                        )
                        .col(ColumnDef::new(Charms::AssetType).string().not_null())
                        .to_owned(),
                )
                .await?;

            // Create indexes on charms table
            manager
                .create_index(
                    Index::create()
                        .name("charms_block_height")
                        .table(Charms::Table)
                        .col(Charms::BlockHeight)
                        .if_not_exists()
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .name("charms_asset_type")
                        .table(Charms::Table)
                        .col(Charms::AssetType)
                        .if_not_exists()
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .name("charms_charmid")
                        .table(Charms::Table)
                        .col(Charms::Charmid)
                        .if_not_exists()
                        .to_owned(),
                )
                .await?;
        }

        if !manager.has_table("transactions").await? {
            // Create transactions table
            manager
                .create_table(
                    Table::create()
                        .table(Transactions::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Transactions::Txid)
                                .string()
                                .not_null()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(Transactions::BlockHeight)
                                .integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Transactions::Ordinal)
                                .big_integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Transactions::Raw)
                                .json_binary()
                                .not_null()
                                .default("{}"),
                        )
                        .col(
                            ColumnDef::new(Transactions::Charm)
                                .json_binary()
                                .not_null()
                                .default("{}"),
                        )
                        .col(
                            ColumnDef::new(Transactions::UpdatedAt)
                                .timestamp()
                                .not_null()
                                .default(Expr::current_timestamp()),
                        )
                        .to_owned(),
                )
                .await?;

            // Create index on transactions block height
            manager
                .create_index(
                    Index::create()
                        .name("transactions_block_height")
                        .table(Transactions::Table)
                        .col(Transactions::BlockHeight)
                        .if_not_exists()
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order
        manager
            .drop_table(Table::drop().table(Transactions::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Charms::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Bookmark::Table).to_owned())
            .await?;

        Ok(())
    }
}

// Bookmark table
#[derive(Iden)]
enum Bookmark {
    Table,
    Hash,
    Height,
}

// Charms table
#[derive(Iden)]
enum Charms {
    Table,
    Txid,
    Charmid,
    BlockHeight,
    Data,
    DateCreated,
    AssetType,
}

// Transactions table
#[derive(Iden)]
enum Transactions {
    Table,
    Txid,
    BlockHeight,
    Ordinal,
    Raw,
    Charm,
    UpdatedAt,
}
