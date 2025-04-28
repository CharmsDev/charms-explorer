use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add status field to bookmark table
        if manager.has_table("bookmark").await? {
            if !manager.has_column("bookmark", "status").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Bookmark::Table)
                            .add_column(
                                ColumnDef::new(Bookmark::Status)
                                    .string()
                                    .not_null()
                                    .default("pending"),
                            )
                            .to_owned(),
                    )
                    .await?;
            }
        }

        // Add status and confirmations fields to transactions table
        if manager.has_table("transactions").await? {
            if !manager.has_column("transactions", "status").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Transactions::Table)
                            .add_column(
                                ColumnDef::new(Transactions::Status)
                                    .string()
                                    .not_null()
                                    .default("pending"),
                            )
                            .to_owned(),
                    )
                    .await?;
            }

            if !manager.has_column("transactions", "confirmations").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Transactions::Table)
                            .add_column(
                                ColumnDef::new(Transactions::Confirmations)
                                    .integer()
                                    .not_null()
                                    .default(0),
                            )
                            .to_owned(),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove status field from bookmark table
        if manager.has_table("bookmark").await? {
            if manager.has_column("bookmark", "status").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Bookmark::Table)
                            .drop_column(Bookmark::Status)
                            .to_owned(),
                    )
                    .await?;
            }
        }

        // Remove status and confirmations fields from transactions table
        if manager.has_table("transactions").await? {
            if manager.has_column("transactions", "status").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Transactions::Table)
                            .drop_column(Transactions::Status)
                            .to_owned(),
                    )
                    .await?;
            }

            if manager.has_column("transactions", "confirmations").await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Transactions::Table)
                            .drop_column(Transactions::Confirmations)
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
}

// Transactions table
#[derive(Iden)]
enum Transactions {
    Table,
    Txid,
    BlockHeight,
    Ordinal,
    Raw,
    Charm,
    UpdatedAt,
    Status,
    Confirmations,
}
