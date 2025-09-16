// Charm database operations implementation

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, PaginatorTrait};

use crate::db::error::DbError;
use crate::entity::charms;
use crate::models::PaginationParams;

/// Repository for charm database operations
pub struct CharmRepository {
    conn: DatabaseConnection,
}

impl CharmRepository {
    /// Creates a new charm repository with database connection
    pub fn new(conn: DatabaseConnection) -> Self {
        CharmRepository { conn }
    }

    /// Returns a reference to the underlying database connection
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.conn
    }

    /// Retrieves a charm by transaction ID
    pub async fn get_by_txid(&self, txid: &str) -> Result<Option<charms::Model>, DbError> {
        charms::Entity::find_by_id(txid.to_string())
            .one(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Finds all charms with matching charm ID
    pub async fn find_by_charmid(&self, charmid: &str) -> Result<Vec<charms::Model>, DbError> {
        charms::Entity::find()
            .filter(charms::Column::Charmid.eq(charmid))
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Finds all charms with matching asset type
    pub async fn find_by_asset_type(
        &self,
        asset_type: &str,
    ) -> Result<Vec<charms::Model>, DbError> {
        charms::Entity::find()
            .filter(charms::Column::AssetType.eq(asset_type))
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }
    
    /// Finds all charms with matching asset type with pagination and sorting
    pub async fn find_by_asset_type_paginated(
        &self,
        asset_type: &str,
        pagination: &PaginationParams,
    ) -> Result<(Vec<charms::Model>, u64), DbError> {
        let mut query = charms::Entity::find()
            .filter(charms::Column::AssetType.eq(asset_type));
        
        // Apply sorting
        query = match pagination.sort.as_str() {
            "oldest" => query.order_by_asc(charms::Column::BlockHeight),
            _ => query.order_by_desc(charms::Column::BlockHeight), // "newest" is default
        };
        
        // Get total count
        let total = query.clone().count(&self.conn).await?;
        
        // Apply pagination
        let paginator = query
            .paginate(&self.conn, pagination.limit);
        
        let charms = paginator
            .fetch_page(pagination.page - 1) // 0-indexed page
            .await?;
        
        Ok((charms, total))
    }

    /// Retrieves all charms with pagination and sorting, optionally filtered by network
    pub async fn get_all_paginated(
        &self,
        pagination: &PaginationParams,
    ) -> Result<(Vec<charms::Model>, u64), DbError> {
        let mut query = charms::Entity::find();
        
        // Apply sorting
        query = match pagination.sort.as_str() {
            "oldest" => query.order_by_asc(charms::Column::BlockHeight),
            _ => query.order_by_desc(charms::Column::BlockHeight), // "newest" is default
        };
        
        // Get total count
        let total = query.clone().count(&self.conn).await?;
        
        // Apply pagination
        let paginator = query
            .paginate(&self.conn, pagination.limit);
        
        let charms = paginator
            .fetch_page(pagination.page - 1) // 0-indexed page
            .await?;
        
        Ok((charms, total))
    }

    /// Retrieves all charms with pagination, sorting, and network filtering
    pub async fn get_all_paginated_by_network(
        &self,
        pagination: &PaginationParams,
        network: Option<&str>,
    ) -> Result<(Vec<charms::Model>, u64), DbError> {
        let mut query = charms::Entity::find();
        
        // Apply network filter if provided
        if let Some(network) = network {
            query = query.filter(charms::Column::Network.eq(network));
        }
        
        // Apply sorting
        query = match pagination.sort.as_str() {
            "oldest" => query.order_by_asc(charms::Column::BlockHeight),
            _ => query.order_by_desc(charms::Column::BlockHeight), // "newest" is default
        };
        
        // Get total count
        let total = query.clone().count(&self.conn).await?;
        
        // Apply pagination
        let paginator = query
            .paginate(&self.conn, pagination.limit);
        
        let charms = paginator
            .fetch_page(pagination.page - 1) // 0-indexed page
            .await?;
        
        Ok((charms, total))
    }
    
    /// Retrieves all charms ordered by descending block height (legacy method)
    pub async fn get_all(&self) -> Result<Vec<charms::Model>, DbError> {
        charms::Entity::find()
            .order_by_desc(charms::Column::BlockHeight)
            .all(&self.conn)
            .await
            .map_err(Into::into)
    }

    /// Retrieves all charm IDs, filtered by asset type if provided
    pub async fn get_charm_numbers_by_type(
        &self,
        asset_type: Option<&str>,
    ) -> Result<Vec<String>, DbError> {
        let mut query = charms::Entity::find();

        if let Some(asset_type) = asset_type {
            query = query.filter(charms::Column::AssetType.eq(asset_type));
        }

        let charms = query
            .order_by_desc(charms::Column::BlockHeight)
            .all(&self.conn)
            .await?;

        let charm_numbers = charms.into_iter().map(|c| c.charmid).collect();

        Ok(charm_numbers)
    }
}
