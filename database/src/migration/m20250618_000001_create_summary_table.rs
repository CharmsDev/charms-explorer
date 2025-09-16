use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create summary table for optimized stats queries
        manager
            .create_table(
                Table::create()
                    .table(Summary::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Summary::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Summary::Network).string().not_null())
                    .col(ColumnDef::new(Summary::LastProcessedBlock).integer().not_null().default(0))
                    .col(ColumnDef::new(Summary::LatestConfirmedBlock).integer().not_null().default(0))
                    .col(ColumnDef::new(Summary::TotalCharms).big_integer().not_null().default(0))
                    .col(ColumnDef::new(Summary::TotalTransactions).big_integer().not_null().default(0))
                    .col(ColumnDef::new(Summary::ConfirmedTransactions).big_integer().not_null().default(0))
                    .col(ColumnDef::new(Summary::ConfirmationRate).integer().not_null().default(0))
                    .col(ColumnDef::new(Summary::NftCount).big_integer().not_null().default(0))
                    .col(ColumnDef::new(Summary::TokenCount).big_integer().not_null().default(0))
                    .col(ColumnDef::new(Summary::DappCount).big_integer().not_null().default(0))
                    .col(ColumnDef::new(Summary::OtherCount).big_integer().not_null().default(0))
                    .col(ColumnDef::new(Summary::BitcoinNodeStatus).string().not_null().default("unknown"))
                    .col(ColumnDef::new(Summary::BitcoinNodeBlockCount).big_integer().not_null().default(0))
                    .col(ColumnDef::new(Summary::BitcoinNodeBestBlockHash).string().not_null().default("unknown"))
                    .col(ColumnDef::new(Summary::LastUpdated).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Summary::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Summary::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        // Create unique index for summary table
        manager
            .create_index(
                Index::create()
                    .name("idx_summary_network")
                    .table(Summary::Table)
                    .col(Summary::Network)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        // Insert initial summary rows for both networks using INSERT ... ON CONFLICT DO NOTHING
        manager
            .get_connection()
            .execute_unprepared(
                "INSERT INTO summary (network, last_updated, created_at, updated_at) 
                 VALUES ('mainnet', NOW(), NOW(), NOW()) 
                 ON CONFLICT (network) DO NOTHING;"
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "INSERT INTO summary (network, last_updated, created_at, updated_at) 
                 VALUES ('testnet4', NOW(), NOW(), NOW()) 
                 ON CONFLICT (network) DO NOTHING;"
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Summary::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Summary {
    Table,
    Id,
    Network,
    LastProcessedBlock,
    LatestConfirmedBlock,
    TotalCharms,
    TotalTransactions,
    ConfirmedTransactions,
    ConfirmationRate,
    NftCount,
    TokenCount,
    DappCount,
    OtherCount,
    BitcoinNodeStatus,
    BitcoinNodeBlockCount,
    BitcoinNodeBestBlockHash,
    LastUpdated,
    CreatedAt,
    UpdatedAt,
}
