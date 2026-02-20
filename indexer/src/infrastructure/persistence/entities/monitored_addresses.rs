use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "monitored_addresses")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub address: String,
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub network: String,
    #[sea_orm(column_type = "Text")]
    pub source: String,
    #[sea_orm(nullable)]
    pub seeded_at: Option<DateTime<Utc>>,
    #[sea_orm(nullable)]
    pub seed_height: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
