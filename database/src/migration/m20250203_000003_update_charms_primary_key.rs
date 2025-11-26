use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // [RJJ-S01] Update charms table to use composite primary key (txid, vout)
        // and remove charmid column
        
        // Step 1: Drop the old primary key constraint
        manager
            .drop_index(
                Index::drop()
                    .name("charms_pkey")
                    .table(Charms::Table)
                    .to_owned(),
            )
            .await?;

        // Step 2: Drop charmid column (no longer needed)
        manager
            .alter_table(
                Table::alter()
                    .table(Charms::Table)
                    .drop_column(Charms::Charmid)
                    .to_owned(),
            )
            .await?;

        // Step 3: Drop the old charmid index
        manager
            .drop_index(
                Index::drop()
                    .name("charms_charmid")
                    .table(Charms::Table)
                    .to_owned(),
            )
            .await?;

        // Step 4: Create new composite primary key (txid, vout)
        manager
            .create_index(
                Index::create()
                    .name("charms_pkey")
                    .table(Charms::Table)
                    .col(Charms::Txid)
                    .col(Charms::Vout)
                    .primary()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Step 1: Drop composite primary key
        manager
            .drop_index(
                Index::drop()
                    .name("charms_pkey")
                    .table(Charms::Table)
                    .to_owned(),
            )
            .await?;

        // Step 2: Add charmid column back
        manager
            .alter_table(
                Table::alter()
                    .table(Charms::Table)
                    .add_column(
                        ColumnDef::new(Charms::Charmid)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;

        // Step 3: Recreate charmid index
        manager
            .create_index(
                Index::create()
                    .name("charms_charmid")
                    .table(Charms::Table)
                    .col(Charms::Charmid)
                    .to_owned(),
            )
            .await?;

        // Step 4: Recreate old primary key on txid only
        manager
            .create_index(
                Index::create()
                    .name("charms_pkey")
                    .table(Charms::Table)
                    .col(Charms::Txid)
                    .primary()
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
    Charmid,
    Vout,
}
