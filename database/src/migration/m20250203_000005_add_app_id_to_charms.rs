use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // [RJJ-S01] Add app_id column to charms table
        // app_id identifies the charm type: t/... (token), n/... (nft), or other
        manager
            .alter_table(
                Table::alter()
                    .table(Charms::Table)
                    .add_column(
                        ColumnDef::new(Charms::AppId)
                            .string()
                            .null()
                            .default("other"),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on app_id for faster queries
        manager
            .create_index(
                Index::create()
                    .name("idx_charms_app_id")
                    .table(Charms::Table)
                    .col(Charms::AppId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop index
        manager
            .drop_index(
                Index::drop()
                    .name("idx_charms_app_id")
                    .table(Charms::Table)
                    .to_owned(),
            )
            .await?;

        // Drop column
        manager
            .alter_table(
                Table::alter()
                    .table(Charms::Table)
                    .drop_column(Charms::AppId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Charms {
    Table,
    AppId,
}
