use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add blockchain and network fields to charms table
        if manager.has_table("charms").await? {
            // Add blockchain field
            if !manager.has_column("charms", "blockchain").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Charms::Table)
                            .add_column(
                                ColumnDef::new(Charms::Blockchain)
                                    .string()
                                    .not_null()
                                    .default("Bitcoin"),
                            )
                            .to_owned(),
                    )
                    .await?;
            }

            // Add network field
            if !manager.has_column("charms", "network").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Charms::Table)
                            .add_column(
                                ColumnDef::new(Charms::Network)
                                    .string()
                                    .not_null()
                                    .default("testnet4"),
                            )
                            .to_owned(),
                    )
                    .await?;
            }

            // Create index on blockchain and network
            manager
                .create_index(
                    Index::create()
                        .name("charms_blockchain_network")
                        .table(Charms::Table)
                        .col(Charms::Blockchain)
                        .col(Charms::Network)
                        .if_not_exists()
                        .to_owned(),
                )
                .await?;
        }

        // Add blockchain and network fields to transactions table
        if manager.has_table("transactions").await? {
            // Add blockchain field
            if !manager.has_column("transactions", "blockchain").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Transactions::Table)
                            .add_column(
                                ColumnDef::new(Transactions::Blockchain)
                                    .string()
                                    .not_null()
                                    .default("Bitcoin"),
                            )
                            .to_owned(),
                    )
                    .await?;
            }

            // Add network field
            if !manager.has_column("transactions", "network").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Transactions::Table)
                            .add_column(
                                ColumnDef::new(Transactions::Network)
                                    .string()
                                    .not_null()
                                    .default("testnet4"),
                            )
                            .to_owned(),
                    )
                    .await?;
            }

            // Create index on blockchain and network
            manager
                .create_index(
                    Index::create()
                        .name("transactions_blockchain_network")
                        .table(Transactions::Table)
                        .col(Transactions::Blockchain)
                        .col(Transactions::Network)
                        .if_not_exists()
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove blockchain and network fields from charms table
        if manager.has_table("charms").await? {
            // Drop index first
            manager
                .drop_index(
                    Index::drop()
                        .name("charms_blockchain_network")
                        .table(Charms::Table)
                        .to_owned(),
                )
                .await?;

            // Drop network column
            if manager.has_column("charms", "network").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Charms::Table)
                            .drop_column(Charms::Network)
                            .to_owned(),
                    )
                    .await?;
            }

            // Drop blockchain column
            if manager.has_column("charms", "blockchain").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Charms::Table)
                            .drop_column(Charms::Blockchain)
                            .to_owned(),
                    )
                    .await?;
            }
        }

        // Remove blockchain and network fields from transactions table
        if manager.has_table("transactions").await? {
            // Drop index first
            manager
                .drop_index(
                    Index::drop()
                        .name("transactions_blockchain_network")
                        .table(Transactions::Table)
                        .to_owned(),
                )
                .await?;

            // Drop network column
            if manager.has_column("transactions", "network").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Transactions::Table)
                            .drop_column(Transactions::Network)
                            .to_owned(),
                    )
                    .await?;
            }

            // Drop blockchain column
            if manager.has_column("transactions", "blockchain").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Transactions::Table)
                            .drop_column(Transactions::Blockchain)
                            .to_owned(),
                    )
                    .await?;
            }
        }

        Ok(())
    }
}

// Charms table
#[derive(Iden)]
enum Charms {
    Table,
    Blockchain,
    Network,
}

// Transactions table
#[derive(Iden)]
enum Transactions {
    Table,
    Blockchain,
    Network,
}
