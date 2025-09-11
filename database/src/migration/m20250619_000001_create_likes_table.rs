use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250619_000001_create_likes_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Likes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Likes::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Likes::CharmId).string().not_null())
                    .col(ColumnDef::new(Likes::UserId).integer().not_null())
                    .col(
                        ColumnDef::new(Likes::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .index(
                        Index::create()
                            .name("idx_likes_charm_id")
                            .col(Likes::CharmId),
                    )
                    .index(
                        Index::create()
                            .name("idx_likes_user_id")
                            .col(Likes::UserId),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("idx_likes_charm_user_unique")
                            .col(Likes::CharmId)
                            .col(Likes::UserId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Likes::Table).to_owned())
            .await
    }
}

/// Likes table definition
#[derive(Iden)]
enum Likes {
    Table,
    Id,
    CharmId,
    UserId,
    CreatedAt,
}
