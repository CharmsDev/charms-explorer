use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add network field to bookmark table
        if manager.has_table("bookmark").await? {
            if !manager.has_column("bookmark", "network").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Bookmark::Table)
                            .add_column(
                                ColumnDef::new(Bookmark::Network)
                                    .string()
                                    .not_null()
                                    .default("Bitcoin-testnet4"),
                            )
                            .to_owned(),
                    )
                    .await?;
            }

            // Create a composite primary key on hash and network
            // First, drop the existing primary key
            manager
                .get_connection()
                .execute_unprepared("ALTER TABLE bookmark DROP CONSTRAINT IF EXISTS bookmark_pkey;")
                .await?;

            // Then, create a new primary key on hash and network
            manager
                .get_connection()
                .execute_unprepared("ALTER TABLE bookmark ADD PRIMARY KEY (hash, network);")
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove network field from bookmark table
        if manager.has_table("bookmark").await? {
            // First, drop the composite primary key
            manager
                .get_connection()
                .execute_unprepared("ALTER TABLE bookmark DROP CONSTRAINT IF EXISTS bookmark_pkey;")
                .await?;

            // Then, recreate the primary key on hash only
            manager
                .get_connection()
                .execute_unprepared("ALTER TABLE bookmark ADD PRIMARY KEY (hash);")
                .await?;

            // Finally, drop the network column
            if manager.has_column("bookmark", "network").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Bookmark::Table)
                            .drop_column(Bookmark::Network)
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
    Network,
}
