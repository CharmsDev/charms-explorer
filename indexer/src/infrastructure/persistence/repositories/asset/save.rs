//! Save operations for asset repository

use chrono::{DateTime, FixedOffset, Utc};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, NotSet, QueryFilter, Set};
use serde_json::Value;

use super::helpers;
use crate::domain::models::Asset;
use crate::domain::models::asset_metadata::{AssetMetadata, DEFAULT_DECIMALS};
use crate::infrastructure::persistence::entities::{assets, prelude::*};
use crate::infrastructure::persistence::error::DbError;

/// [RJJ-SUPPLY] Save or update asset with correct supply logic
/// [RJJ-DECIMALS] Extract and store decimals from NFT metadata
///
/// Supply Rules:
/// - NFT creation: total_supply = 0 (NFT itself has implicit supply of 1)
/// - Token creation: increment parent NFT's supply (or create token asset if no NFT exists)
/// - Supply only tracks child tokens, not the NFT itself
///
/// Decimals Rules:
/// - NFT: Extract decimals from metadata (default: 8)
/// - Token: Use parent NFT's decimals (default: 8 if no NFT)
pub async fn save_or_update_asset(
    db: &DatabaseConnection,
    asset: &Asset,
    amount: i64,
) -> Result<(), DbError> {
    let asset_type = asset.asset_type.as_str();

    // Extract hash for NFT-Token matching
    let hash = helpers::extract_hash_from_app_id(&asset.app_id);

    match asset_type {
        "nft" => {
            // NFT creation: check if already exists
            let existing_nft = Assets::find()
                .filter(assets::Column::AppId.eq(&asset.app_id))
                .one(db)
                .await
                .map_err(|e| DbError::SeaOrmError(e))?;

            if existing_nft.is_none() {
                // Extract metadata from NFT data
                let metadata = AssetMetadata::from_nft_data(&asset.data);

                // Create new NFT with supply = 0 and extracted decimals
                // Note: is_reference_nft starts as false, will be set to true when a token is found
                let active_model = assets::ActiveModel {
                    id: NotSet,
                    app_id: Set(asset.app_id.clone()),
                    txid: Set(asset.txid.clone()),
                    vout_index: Set(asset.vout_index),
                    charm_id: Set(asset.charm_id.clone()),
                    block_height: Set(asset.block_height as i32),
                    date_created: Set(DateTime::<FixedOffset>::from(
                        DateTime::<Utc>::from_naive_utc_and_offset(asset.date_created, Utc),
                    )),
                    data: Set(asset.data.clone()),
                    asset_type: Set(asset.asset_type.clone()),
                    blockchain: Set(asset.blockchain.clone()),
                    network: Set(asset.network.clone()),
                    name: Set(metadata.name),
                    symbol: Set(metadata.symbol),
                    description: Set(metadata.description),
                    image_url: Set(metadata.image_url),
                    total_supply: Set(Some(Decimal::ZERO)), // NFT supply starts at 0
                    decimals: Set(metadata.decimals as i16), // [RJJ-DECIMALS]
                    is_reference_nft: Set(false), // Will be set to true when a token is found
                    created_at: Set(Utc::now().into()),
                    updated_at: Set(Utc::now().into()),
                };

                Assets::insert(active_model)
                    .exec(db)
                    .await
                    .map_err(|e| DbError::SeaOrmError(e))?;
            }
            // If NFT already exists, do nothing (idempotent)
        }
        "token" => {
            // Token creation: find parent NFT (n/HASH/...)
            // Use LIKE pattern because app_id format is n/{hash}/{txid}:{vout}
            let parent_nft_pattern = format!("n/{}/%", hash);
            let parent_nft = Assets::find()
                .filter(assets::Column::AssetType.eq("nft"))
                .filter(assets::Column::AppId.like(&parent_nft_pattern))
                .one(db)
                .await
                .map_err(|e| DbError::SeaOrmError(e))?;

            // Get decimals and metadata from parent NFT or use defaults
            // Note: We only inherit name, symbol, description - NOT image_url
            // Image should be fetched from reference NFT on demand to save space
            let (decimals, name, symbol, description, should_mark_nft_as_reference) =
                if let Some(ref nft) = parent_nft {
                    (
                        nft.decimals,
                        nft.name.clone(),
                        nft.symbol.clone(),
                        nft.description.clone(),
                        !nft.is_reference_nft, // Only mark if not already marked
                    )
                } else {
                    (DEFAULT_DECIMALS as i16, None, None, None, false)
                };

            // Mark parent NFT as reference if this is the first token for it
            if should_mark_nft_as_reference {
                if let Some(ref nft) = parent_nft {
                    if let Err(e) = mark_nft_as_reference(db, &nft.app_id).await {
                        crate::utils::logging::log_warning(&format!(
                            "Failed to mark NFT as reference: {}",
                            e
                        ));
                    }
                }
            }

            // ALWAYS create/update token asset record (for Tokens tab display)
            let existing_token = Assets::find()
                .filter(assets::Column::AppId.eq(&asset.app_id))
                .one(db)
                .await
                .map_err(|e| DbError::SeaOrmError(e))?;

            match existing_token {
                Some(existing) => {
                    // Token asset exists, increment supply
                    let old_supply = existing.total_supply.unwrap_or(Decimal::ZERO);
                    let amount_decimal = Decimal::from(amount);
                    let new_supply = old_supply + amount_decimal;

                    let update_model = assets::ActiveModel {
                        id: Set(existing.id),
                        total_supply: Set(Some(new_supply)),
                        updated_at: Set(Utc::now().into()),
                        ..Default::default()
                    };

                    Assets::update(update_model)
                        .exec(db)
                        .await
                        .map_err(|e| DbError::SeaOrmError(e))?;
                }
                None => {
                    // Create new token asset with metadata inherited from parent NFT
                    let active_model = assets::ActiveModel {
                        id: NotSet,
                        app_id: Set(asset.app_id.clone()),
                        txid: Set(asset.txid.clone()),
                        vout_index: Set(asset.vout_index),
                        charm_id: Set(asset.charm_id.clone()),
                        block_height: Set(asset.block_height as i32),
                        date_created: Set(DateTime::<FixedOffset>::from(
                            DateTime::<Utc>::from_naive_utc_and_offset(asset.date_created, Utc),
                        )),
                        data: Set(asset.data.clone()),
                        asset_type: Set(asset.asset_type.clone()),
                        blockchain: Set(asset.blockchain.clone()),
                        network: Set(asset.network.clone()),
                        name: Set(name),               // Inherit from parent NFT
                        symbol: Set(symbol),           // Inherit from parent NFT
                        description: Set(description), // Inherit from parent NFT
                        image_url: Set(None), // NOT inherited - fetch from reference NFT on demand
                        total_supply: Set(Some(Decimal::from(amount))),
                        decimals: Set(decimals), // [RJJ-DECIMALS] Use parent NFT's decimals or default
                        is_reference_nft: Set(false), // Tokens are never reference NFTs
                        created_at: Set(Utc::now().into()),
                        updated_at: Set(Utc::now().into()),
                    };

                    Assets::insert(active_model)
                        .exec(db)
                        .await
                        .map_err(|e| DbError::SeaOrmError(e))?;
                }
            }
        }
        _ => {
            // Other asset types: use simple accumulation with default decimals
            let existing_asset = Assets::find()
                .filter(assets::Column::AppId.eq(&asset.app_id))
                .one(db)
                .await
                .map_err(|e| DbError::SeaOrmError(e))?;

            match existing_asset {
                Some(existing) => {
                    let old_supply = existing.total_supply.unwrap_or(Decimal::ZERO);
                    let amount_decimal = Decimal::from(amount);
                    let new_supply = old_supply + amount_decimal;

                    let update_model = assets::ActiveModel {
                        id: Set(existing.id),
                        total_supply: Set(Some(new_supply)),
                        updated_at: Set(Utc::now().into()),
                        ..Default::default()
                    };

                    Assets::update(update_model)
                        .exec(db)
                        .await
                        .map_err(|e| DbError::SeaOrmError(e))?;
                }
                None => {
                    let active_model = assets::ActiveModel {
                        id: NotSet,
                        app_id: Set(asset.app_id.clone()),
                        txid: Set(asset.txid.clone()),
                        vout_index: Set(asset.vout_index),
                        charm_id: Set(asset.charm_id.clone()),
                        block_height: Set(asset.block_height as i32),
                        date_created: Set(DateTime::<FixedOffset>::from(
                            DateTime::<Utc>::from_naive_utc_and_offset(asset.date_created, Utc),
                        )),
                        data: Set(asset.data.clone()),
                        asset_type: Set(asset.asset_type.clone()),
                        blockchain: Set(asset.blockchain.clone()),
                        network: Set(asset.network.clone()),
                        name: Set(None),
                        symbol: Set(None),
                        description: Set(None),
                        image_url: Set(None),
                        total_supply: Set(Some(Decimal::from(amount))),
                        decimals: Set(DEFAULT_DECIMALS as i16), // [RJJ-DECIMALS] Default for other types
                        is_reference_nft: Set(false),
                        created_at: Set(Utc::now().into()),
                        updated_at: Set(Utc::now().into()),
                    };

                    Assets::insert(active_model)
                        .exec(db)
                        .await
                        .map_err(|e| DbError::SeaOrmError(e))?;
                }
            }
        }
    }

    Ok(())
}

/// Mark an NFT as a reference NFT (has associated tokens)
/// This is called when the first token for this NFT is created
async fn mark_nft_as_reference(db: &DatabaseConnection, nft_app_id: &str) -> Result<(), DbError> {
    let nft = Assets::find()
        .filter(assets::Column::AppId.eq(nft_app_id))
        .one(db)
        .await
        .map_err(|e| DbError::SeaOrmError(e))?;

    if let Some(nft) = nft {
        let update_model = assets::ActiveModel {
            id: Set(nft.id),
            is_reference_nft: Set(true),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };

        Assets::update(update_model)
            .exec(db)
            .await
            .map_err(|e| DbError::SeaOrmError(e))?;
    }

    Ok(())
}

/// Save a single asset to the database (legacy method)
pub async fn save_asset(db: &DatabaseConnection, asset: &Asset) -> Result<(), DbError> {
    // Use the new method with amount = 1 for backward compatibility
    save_or_update_asset(db, asset, 1).await
}

/// Save multiple assets in a batch operation
/// For tokens, inherits metadata from parent NFT if it exists
pub async fn save_batch(
    db: &DatabaseConnection,
    assets: Vec<(
        String, // app_id
        String, // txid
        i32,    // vout_index
        String, // charm_id
        u64,    // block_height
        Value,  // data
        String, // asset_type
        String, // blockchain
        String, // network
    )>,
) -> Result<(), DbError> {
    if assets.is_empty() {
        return Ok(());
    }

    let assets_count = assets.len();

    // Log only for larger batches to reduce noise
    if assets_count > 5 {
        crate::utils::logging::log_info(&format!(
            "Batch saving {} assets to database",
            assets_count
        ));
    }

    let now = Utc::now();

    // Separate NFTs and tokens - NFTs must be inserted first so tokens can find their parent
    let (nfts, tokens): (Vec<_>, Vec<_>) = assets
        .into_iter()
        .partition(|(_, _, _, _, _, _, asset_type, _, _)| asset_type == "nft");

    // Process NFTs first
    for (app_id, txid, vout_index, charm_id, block_height, data, asset_type, blockchain, network) in
        nfts
    {
        let metadata = AssetMetadata::from_nft_data(&data);

        let active_model = assets::ActiveModel {
            id: NotSet,
            app_id: Set(app_id),
            txid: Set(txid),
            vout_index: Set(vout_index),
            charm_id: Set(charm_id),
            block_height: Set(block_height as i32),
            date_created: Set(now.into()),
            data: Set(data),
            asset_type: Set(asset_type),
            blockchain: Set(blockchain),
            network: Set(network),
            name: Set(metadata.name),
            symbol: Set(metadata.symbol),
            description: Set(metadata.description),
            image_url: Set(metadata.image_url),
            total_supply: Set(Some(Decimal::ZERO)), // NFTs start with 0 supply
            decimals: Set(metadata.decimals as i16),
            is_reference_nft: Set(false),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };

        // Insert NFT immediately so tokens can find it
        if let Err(e) = Assets::insert(active_model).exec(db).await {
            crate::utils::logging::log_warning(&format!(
                "NFT insert error (may be duplicate): {}",
                e
            ));
        }
    }

    // Now process tokens - check if they exist and update supply, or create new
    for (app_id, txid, vout_index, charm_id, block_height, data, asset_type, blockchain, network) in
        tokens
    {
        // Extract supply from data.supply field (this is the mint amount)
        let mint_amount = if let Some(supply_value) = data.get("supply") {
            if let Some(supply_i64) = supply_value.as_i64() {
                Decimal::from(supply_i64)
            } else {
                Decimal::from(1)
            }
        } else {
            Decimal::from(1)
        };

        // Check if token already exists
        let existing_token = Assets::find()
            .filter(assets::Column::AppId.eq(&app_id))
            .one(db)
            .await
            .map_err(|e| DbError::SeaOrmError(e))?;

        if let Some(existing) = existing_token {
            // Token exists - add mint amount to existing supply
            let old_supply = existing.total_supply.unwrap_or(Decimal::ZERO);
            let new_supply = old_supply + mint_amount;

            let update_model = assets::ActiveModel {
                id: Set(existing.id),
                total_supply: Set(Some(new_supply)),
                updated_at: Set(now.into()),
                ..Default::default()
            };

            Assets::update(update_model)
                .exec(db)
                .await
                .map_err(|e| DbError::SeaOrmError(e))?;
        } else {
            // Token doesn't exist - create new with inherited metadata from parent NFT
            let hash = helpers::extract_hash_from_app_id(&app_id);
            let parent_nft_pattern = format!("n/{}/%", hash);

            let (name, symbol, description, decimals) = if let Ok(Some(parent_nft)) = Assets::find()
                .filter(assets::Column::AssetType.eq("nft"))
                .filter(assets::Column::AppId.like(&parent_nft_pattern))
                .one(db)
                .await
            {
                // Mark parent NFT as reference if not already marked
                if !parent_nft.is_reference_nft {
                    if let Err(e) = mark_nft_as_reference(db, &parent_nft.app_id).await {
                        crate::utils::logging::log_warning(&format!(
                            "Failed to mark NFT as reference: {}",
                            e
                        ));
                    }
                }
                (
                    parent_nft.name,
                    parent_nft.symbol,
                    parent_nft.description,
                    parent_nft.decimals,
                )
            } else {
                (None, None, None, DEFAULT_DECIMALS as i16)
            };

            let active_model = assets::ActiveModel {
                id: NotSet,
                app_id: Set(app_id),
                txid: Set(txid),
                vout_index: Set(vout_index),
                charm_id: Set(charm_id),
                block_height: Set(block_height as i32),
                date_created: Set(now.into()),
                data: Set(data),
                asset_type: Set(asset_type),
                blockchain: Set(blockchain),
                network: Set(network),
                name: Set(name),
                symbol: Set(symbol),
                description: Set(description),
                image_url: Set(None),
                total_supply: Set(Some(mint_amount)),
                decimals: Set(decimals),
                is_reference_nft: Set(false),
                created_at: Set(now.into()),
                updated_at: Set(now.into()),
            };

            if let Err(e) = Assets::insert(active_model).exec(db).await {
                crate::utils::logging::log_warning(&format!(
                    "Token insert error (may be duplicate): {}",
                    e
                ));
            }
        }
    }

    Ok(())
}

/// Update NFT metadata (name, image_url) directly
pub async fn update_nft_metadata(
    db: &DatabaseConnection,
    app_id: &str,
    name: Option<&str>,
    image_url: Option<&str>,
) -> Result<(), DbError> {
    use sea_orm::QueryFilter;

    let existing = Assets::find()
        .filter(assets::Column::AppId.eq(app_id))
        .one(db)
        .await
        .map_err(|e| DbError::SeaOrmError(e))?;

    if let Some(asset) = existing {
        let mut active: assets::ActiveModel = asset.into();

        if let Some(n) = name {
            active.name = Set(Some(n.to_string()));
        }
        if let Some(img) = image_url {
            active.image_url = Set(Some(img.to_string()));
        }
        active.updated_at = Set(Utc::now().into());

        Assets::update(active)
            .exec(db)
            .await
            .map_err(|e| DbError::SeaOrmError(e))?;
    }

    Ok(())
}
