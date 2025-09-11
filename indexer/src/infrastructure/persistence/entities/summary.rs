//! SeaORM Entity for Summary table

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "summary")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(column_type = "Text", unique)]
    pub network: String,
    pub last_processed_block: i32,
    pub latest_confirmed_block: i32,
    pub total_charms: i64,
    pub total_transactions: i64,
    pub confirmed_transactions: i64,
    pub confirmation_rate: i32,
    pub nft_count: i64,
    pub token_count: i64,
    pub dapp_count: i64,
    pub other_count: i64,
    pub bitcoin_node_status: String,
    pub bitcoin_node_block_count: i64,
    pub bitcoin_node_best_block_hash: String,
    pub last_updated: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
