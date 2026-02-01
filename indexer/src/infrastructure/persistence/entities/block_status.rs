//! SeaORM Entity for block_status table
//! Unified block tracking for indexer control

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "block_status")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub block_height: i32,
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub network: String,
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub blockchain: String,
    pub downloaded: bool,
    pub processed: bool,
    pub confirmed: bool,
    #[sea_orm(column_type = "Text", nullable)]
    pub block_hash: Option<String>,
    pub tx_count: Option<i32>,
    pub charm_count: Option<i32>,
    #[sea_orm(column_type = "TimestampWithTimeZone", nullable)]
    pub downloaded_at: Option<DateTimeWithTimeZone>,
    #[sea_orm(column_type = "TimestampWithTimeZone", nullable)]
    pub processed_at: Option<DateTimeWithTimeZone>,
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub created_at: DateTimeWithTimeZone,
    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
