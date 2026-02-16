//! SeaORM Entity for dex_orders table

use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "dex_orders")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Text")]
    pub order_id: String,

    #[sea_orm(column_type = "Text")]
    pub txid: String,
    pub vout: i32,
    #[sea_orm(nullable)]
    pub block_height: Option<i32>,

    #[sea_orm(column_type = "Text")]
    pub platform: String,

    #[sea_orm(column_type = "Text")]
    pub maker: String,
    #[sea_orm(column_type = "Text")]
    pub side: String,
    #[sea_orm(column_type = "Text")]
    pub exec_type: String,

    pub price_num: i64,
    pub price_den: i64,

    pub amount: i64,
    pub quantity: i64,
    pub filled_amount: i64,
    pub filled_quantity: i64,

    #[sea_orm(column_type = "Text")]
    pub asset_app_id: String,

    #[sea_orm(column_type = "Text", nullable)]
    pub scrolls_address: Option<String>,

    #[sea_orm(column_type = "Text")]
    pub status: String,

    #[sea_orm(column_type = "Text", nullable)]
    pub parent_order_id: Option<String>,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

    #[sea_orm(column_type = "Text")]
    pub blockchain: String,
    #[sea_orm(column_type = "Text")]
    pub network: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
