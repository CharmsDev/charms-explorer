use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .add_column(ColumnDef::new(Assets::CardanoPolicyId).string().null())
                    .add_column(ColumnDef::new(Assets::CardanoAssetName).string().null())
                    .add_column(ColumnDef::new(Assets::CardanoFingerprint).string().null())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Assets::Table)
                    .drop_column(Assets::CardanoPolicyId)
                    .drop_column(Assets::CardanoAssetName)
                    .drop_column(Assets::CardanoFingerprint)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Assets {
    Table,
    CardanoPolicyId,
    CardanoAssetName,
    CardanoFingerprint,
}
