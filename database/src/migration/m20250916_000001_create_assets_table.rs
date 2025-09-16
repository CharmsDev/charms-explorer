use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create assets table for app_id based indexing
        manager
            .create_table(
                Table::create()
                    .table(Assets::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Assets::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Assets::AppId).string().not_null()) // Primary index - can start with n/, t/, etc.
                    .col(ColumnDef::new(Assets::Txid).string().not_null()) // Transaction ID
                    .col(ColumnDef::new(Assets::VoutIndex).integer().not_null()) // UTXO index
                    .col(ColumnDef::new(Assets::CharmId).string().not_null()) // Reference to charms table
                    .col(ColumnDef::new(Assets::BlockHeight).integer().not_null())
                    .col(ColumnDef::new(Assets::DateCreated).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(Assets::Data).json().not_null().default("{}"))
                    .col(ColumnDef::new(Assets::AssetType).string().not_null())
                    .col(ColumnDef::new(Assets::Blockchain).string().not_null().default("Bitcoin"))
                    .col(ColumnDef::new(Assets::Network).string().not_null().default("testnet4"))
                    .col(ColumnDef::new(Assets::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(Assets::UpdatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // Create primary index on app_id (unique identifier)
        manager
            .create_index(
                Index::create()
                    .name("idx_assets_app_id")
                    .table(Assets::Table)
                    .col(Assets::AppId)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create index on txid and vout_index for UTXO lookup
        manager
            .create_index(
                Index::create()
                    .name("idx_assets_utxo")
                    .table(Assets::Table)
                    .col(Assets::Txid)
                    .col(Assets::VoutIndex)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create index on charm_id for foreign key relationship
        manager
            .create_index(
                Index::create()
                    .name("idx_assets_charm_id")
                    .table(Assets::Table)
                    .col(Assets::CharmId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create index on blockchain and network
        manager
            .create_index(
                Index::create()
                    .name("idx_assets_blockchain_network")
                    .table(Assets::Table)
                    .col(Assets::Blockchain)
                    .col(Assets::Network)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create index on block_height for efficient querying
        manager
            .create_index(
                Index::create()
                    .name("idx_assets_block_height")
                    .table(Assets::Table)
                    .col(Assets::BlockHeight)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Create index on asset_type
        manager
            .create_index(
                Index::create()
                    .name("idx_assets_asset_type")
                    .table(Assets::Table)
                    .col(Assets::AssetType)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Assets::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Assets {
    Table,
    Id,
    AppId,
    Txid,
    VoutIndex,
    CharmId,
    BlockHeight,
    DateCreated,
    Data,
    AssetType,
    Blockchain,
    Network,
    CreatedAt,
    UpdatedAt,
}
