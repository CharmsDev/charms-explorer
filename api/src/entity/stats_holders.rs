// Stats holders entity - Aggregated holder statistics per asset
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "stats_holders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(column_type = "Text")]
    pub app_id: String,
    #[sea_orm(column_type = "Text")]
    pub address: String,
    pub total_amount: i64,
    pub charm_count: i32,
    pub first_seen_block: i32,
    pub last_updated_block: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
