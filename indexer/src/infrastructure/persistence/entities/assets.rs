//! Asset entity for SeaORM

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "assets")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub app_id: String,
    pub txid: String,
    pub vout_index: i32,
    pub charm_id: String,
    pub block_height: i32,
    pub date_created: DateTimeWithTimeZone,
    pub data: Json,
    pub asset_type: String,
    pub blockchain: String,
    pub network: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
