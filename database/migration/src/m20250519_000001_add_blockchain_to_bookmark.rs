use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add blockchain field to bookmark table
        if manager.has_table("bookmark").await? {
            // Add blockchain field
            if !manager.has_column("bookmark", "blockchain").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Bookmark::Table)
                            .add_column(
                                ColumnDef::new(Bookmark::Blockchain)
                                    .string()
                                    .not_null()
                                    .default("Bitcoin"),
                            )
                            .to_owned(),
                    )
                    .await?;
            }

            // Create index on blockchain and network
            manager
                .create_index(
                    Index::create()
                        .name("bookmark_blockchain_network")
                        .table(Bookmark::Table)
                        .col(Bookmark::Blockchain)
                        .col(Bookmark::Network)
                        .if_not_exists()
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove blockchain field from bookmark table
        if manager.has_table("bookmark").await? {
            // Drop index first
            manager
                .drop_index(
                    Index::drop()
                        .name("bookmark_blockchain_network")
                        .table(Bookmark::Table)
                        .to_owned(),
                )
                .await?;

            // Drop blockchain column
            if manager.has_column("bookmark", "blockchain").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Bookmark::Table)
                            .drop_column(Bookmark::Blockchain)
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
    Blockchain,
    Network,
}
