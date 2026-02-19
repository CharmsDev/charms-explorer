use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create address_utxos table
        manager
            .create_table(
                Table::create()
                    .table(AddressUtxos::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AddressUtxos::Txid).string_len(64).not_null())
                    .col(ColumnDef::new(AddressUtxos::Vout).integer().not_null())
                    .col(ColumnDef::new(AddressUtxos::Network).string_len(10).not_null().default("mainnet"))
                    .col(ColumnDef::new(AddressUtxos::Address).string_len(62).not_null())
                    .col(ColumnDef::new(AddressUtxos::Value).big_integer().not_null())
                    .col(ColumnDef::new(AddressUtxos::ScriptPubkey).string_len(140).not_null().default(""))
                    .col(ColumnDef::new(AddressUtxos::BlockHeight).integer().not_null())
                    .primary_key(
                        Index::create()
                            .col(AddressUtxos::Txid)
                            .col(AddressUtxos::Vout)
                            .col(AddressUtxos::Network),
                    )
                    .to_owned(),
            )
            .await?;

        // Index for wallet lookups: address + network
        manager
            .create_index(
                Index::create()
                    .name("idx_address_utxos_address")
                    .table(AddressUtxos::Table)
                    .col(AddressUtxos::Address)
                    .col(AddressUtxos::Network)
                    .to_owned(),
            )
            .await?;

        // Index for block-level operations (reindex/rollback)
        manager
            .create_index(
                Index::create()
                    .name("idx_address_utxos_block")
                    .table(AddressUtxos::Table)
                    .col(AddressUtxos::BlockHeight)
                    .col(AddressUtxos::Network)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AddressUtxos::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum AddressUtxos {
    Table,
    Txid,
    Vout,
    Network,
    Address,
    Value,
    ScriptPubkey,
    BlockHeight,
}
