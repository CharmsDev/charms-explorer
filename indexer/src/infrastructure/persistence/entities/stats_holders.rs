// [RJJ-STATS-HOLDERS] Stats holders entity for indexer
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "stats_holders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub app_id: String,
    pub address: String,
    pub total_amount: i64,
    pub charm_count: i32,
    pub first_seen_block: i32,
    pub last_updated_block: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
