use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add last_updated_at field to bookmark table
        if manager.has_table("bookmark").await? {
            if !manager.has_column("bookmark", "last_updated_at").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Bookmark::Table)
                            .add_column(
                                ColumnDef::new(Bookmark::LastUpdatedAt)
                                    .timestamp_with_time_zone()
                                    .not_null()
                                    .default(Expr::current_timestamp()),
                            )
                            .to_owned(),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove last_updated_at field from bookmark table
        if manager.has_table("bookmark").await? {
            if manager.has_column("bookmark", "last_updated_at").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Bookmark::Table)
                            .drop_column(Bookmark::LastUpdatedAt)
                            .to_owned(),
                    )
                    .await?;
            }
        }

        Ok(())
    }
}

// Bookmark table
#[derive(Iden)]
enum Bookmark {
    Table,
    Hash,
    Height,
    Status,
    LastUpdatedAt,
}
