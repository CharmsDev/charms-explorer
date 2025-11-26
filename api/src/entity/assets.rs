use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "assets")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub app_id: String,
    pub txid: String,
    pub vout_index: i32,
    pub charm_id: String,
    pub block_height: i32,
    pub date_created: chrono::DateTime<chrono::Utc>,
    pub data: serde_json::Value,
    pub asset_type: String,
    pub blockchain: String,
    pub network: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub total_supply: Option<Decimal>,
    pub decimals: i16, // Dynamic decimal precision (default: 8)
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
