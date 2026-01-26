//! Repository for DEX orders operations

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::domain::services::dex::{DexOperation, DexOrder, ExecType, OrderSide};
use crate::infrastructure::persistence::entities::dex_orders;
use crate::infrastructure::persistence::error::DbError;

/// Repository for DEX orders operations
#[derive(Clone, Debug)]
pub struct DexOrdersRepository {
    conn: DatabaseConnection,
}

impl DexOrdersRepository {
    /// Create a new DexOrdersRepository
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Save a DEX order
    pub async fn save_order(
        &self,
        txid: &str,
        vout: i32,
        block_height: Option<u64>,
        order: &DexOrder,
        operation: &DexOperation,
        platform: &str,
        blockchain: &str,
        network: &str,
    ) -> Result<(), DbError> {
        let order_id = format!("{}:{}", txid, vout);
        let now = chrono::Utc::now().naive_utc();

        let (side_str, exec_type_str) = (
            match order.side {
                OrderSide::Ask => "ask",
                OrderSide::Bid => "bid",
            },
            match &order.exec_type {
                ExecType::AllOrNone => "all_or_none",
                ExecType::Partial { .. } => "partial",
            },
        );

        // Determine initial status based on operation
        let status = match operation {
            DexOperation::CreateAskOrder | DexOperation::CreateBidOrder => "open",
            DexOperation::PartialFill => "partial",
            DexOperation::FulfillAsk | DexOperation::FulfillBid => "filled",
            DexOperation::CancelOrder => "cancelled",
        };

        // Get parent order ID for partial fills
        let parent_order_id = if let ExecType::Partial { from } = &order.exec_type {
            from.clone()
        } else {
            None
        };

        let model = dex_orders::ActiveModel {
            order_id: Set(order_id),
            txid: Set(txid.to_string()),
            vout: Set(vout),
            block_height: Set(block_height.map(|h| h as i32)),
            platform: Set(platform.to_string()),
            maker: Set(order.maker.clone()),
            side: Set(side_str.to_string()),
            exec_type: Set(exec_type_str.to_string()),
            price_num: Set(order.price.0 as i64),
            price_den: Set(order.price.1 as i64),
            amount: Set(order.amount as i64),
            quantity: Set(order.quantity as i64),
            filled_amount: Set(0),
            filled_quantity: Set(0),
            asset_app_id: Set(order.asset_app_id.clone()),
            scrolls_address: Set(order.scrolls_address.clone()),
            status: Set(status.to_string()),
            parent_order_id: Set(parent_order_id),
            created_at: Set(now),
            updated_at: Set(now),
            blockchain: Set(blockchain.to_string()),
            network: Set(network.to_string()),
        };

        match model.insert(&self.conn).await {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.to_string().contains("duplicate key") {
                    Ok(()) // Order already exists
                } else {
                    Err(e.into())
                }
            }
        }
    }

    /// Get order by ID
    pub async fn get_by_id(&self, order_id: &str) -> Result<Option<dex_orders::Model>, DbError> {
        let result = dex_orders::Entity::find_by_id(order_id.to_string())
            .one(&self.conn)
            .await?;
        Ok(result)
    }

    /// Find orders by asset
    pub async fn find_by_asset(&self, asset_app_id: &str) -> Result<Vec<dex_orders::Model>, DbError> {
        let results = dex_orders::Entity::find()
            .filter(dex_orders::Column::AssetAppId.eq(asset_app_id))
            .order_by_desc(dex_orders::Column::CreatedAt)
            .all(&self.conn)
            .await?;
        Ok(results)
    }

    /// Find open orders by asset (for orderbook)
    pub async fn find_open_orders_by_asset(
        &self,
        asset_app_id: &str,
    ) -> Result<Vec<dex_orders::Model>, DbError> {
        let results = dex_orders::Entity::find()
            .filter(dex_orders::Column::AssetAppId.eq(asset_app_id))
            .filter(dex_orders::Column::Status.eq("open"))
            .order_by_asc(dex_orders::Column::PriceNum)
            .all(&self.conn)
            .await?;
        Ok(results)
    }

    /// Find orders by maker address
    pub async fn find_by_maker(&self, maker: &str) -> Result<Vec<dex_orders::Model>, DbError> {
        let results = dex_orders::Entity::find()
            .filter(dex_orders::Column::Maker.eq(maker))
            .order_by_desc(dex_orders::Column::CreatedAt)
            .all(&self.conn)
            .await?;
        Ok(results)
    }

    /// Update order status
    pub async fn update_status(&self, order_id: &str, status: &str) -> Result<(), DbError> {
        if let Some(order) = dex_orders::Entity::find_by_id(order_id.to_string())
            .one(&self.conn)
            .await?
        {
            let mut active_model: dex_orders::ActiveModel = order.into();
            active_model.status = Set(status.to_string());
            active_model.updated_at = Set(chrono::Utc::now().naive_utc());
            active_model.update(&self.conn).await?;
        }
        Ok(())
    }

    /// Count orders by platform
    pub async fn count_by_platform(&self, platform: &str) -> Result<u64, DbError> {
        use sea_orm::PaginatorTrait;
        let count = dex_orders::Entity::find()
            .filter(dex_orders::Column::Platform.eq(platform))
            .count(&self.conn)
            .await?;
        Ok(count)
    }
}
