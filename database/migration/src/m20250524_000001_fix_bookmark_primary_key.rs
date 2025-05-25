use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Fix the bookmark table primary key to include blockchain field
        if manager.has_table("bookmark").await? {
            // Drop the existing primary key constraint
            manager
                .get_connection()
                .execute_unprepared("ALTER TABLE bookmark DROP CONSTRAINT IF EXISTS bookmark_pkey;")
                .await?;

            // Create a new composite primary key on hash, network, and blockchain
            manager
                .get_connection()
                .execute_unprepared(
                    "ALTER TABLE bookmark ADD PRIMARY KEY (hash, network, blockchain);",
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert to the previous primary key on hash and network only
        if manager.has_table("bookmark").await? {
            // Drop the current primary key constraint
            manager
                .get_connection()
                .execute_unprepared("ALTER TABLE bookmark DROP CONSTRAINT IF EXISTS bookmark_pkey;")
                .await?;

            // Recreate the previous primary key on hash and network
            manager
                .get_connection()
                .execute_unprepared("ALTER TABLE bookmark ADD PRIMARY KEY (hash, network);")
                .await?;
        }

        Ok(())
    }
}

// Bookmark table
#[derive(Iden)]
enum Bookmark {
    Table,
    Hash,
    Network,
    Blockchain,
}
