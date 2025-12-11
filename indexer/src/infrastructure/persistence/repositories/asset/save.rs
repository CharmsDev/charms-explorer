//! Save operations for asset repository

use chrono::{DateTime, FixedOffset, Utc};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, NotSet, QueryFilter, Set};
use serde_json::Value;

use super::helpers;
use crate::domain::models::asset_metadata::{AssetMetadata, DEFAULT_DECIMALS};
use crate::domain::models::Asset;
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

                // [DEBUG] Log NFT metadata extraction
                println!("🔍 [NFT METADATA DEBUG]");
                println!("   app_id: {}", asset.app_id);
                println!("   name: {:?}", metadata.name);
                println!("   symbol: {:?}", metadata.symbol);
                println!("   description: {:?}", metadata.description);
                println!("   image_url: {:?}", metadata.image_url);
                println!("   decimals: {}", metadata.decimals);
                println!(
                    "   data JSON: {}",
                    serde_json::to_string_pretty(&asset.data)
                        .unwrap_or_else(|_| "ERROR".to_string())
                );

                // Pause execution for debugging
                println!("⏸️  Paused! Press ENTER to continue...");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();

                // Create new NFT with supply = 0 and extracted decimals
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
            // Token creation: find parent NFT (n/HASH)
            let parent_nft_app_id = format!("n/{}", hash);
            let parent_nft = Assets::find()
                .filter(assets::Column::AppId.eq(&parent_nft_app_id))
                .one(db)
                .await
                .map_err(|e| DbError::SeaOrmError(e))?;

            // Get decimals and metadata from parent NFT or use defaults
            let (decimals, name, symbol, description, image_url) = if let Some(ref nft) = parent_nft
            {
                (
                    nft.decimals,
                    nft.name.clone(),
                    nft.symbol.clone(),
                    nft.description.clone(),
                    nft.image_url.clone(),
                )
            } else {
                (DEFAULT_DECIMALS as i16, None, None, None, None)
            };

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
                        image_url: Set(image_url),     // Inherit from parent NFT
                        total_supply: Set(Some(Decimal::from(amount))),
                        decimals: Set(decimals), // [RJJ-DECIMALS] Use parent NFT's decimals or default
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

/// Save a single asset to the database (legacy method)
pub async fn save_asset(db: &DatabaseConnection, asset: &Asset) -> Result<(), DbError> {
    // Use the new method with amount = 1 for backward compatibility
    save_or_update_asset(db, asset, 1).await
}

/// Save multiple assets in a batch operation
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
        println!("💾 Batch saving {} assets to database", assets_count);
    }

    let now = Utc::now();
    let active_models: Vec<assets::ActiveModel> = assets
        .into_iter()
        .map(
            |(
                app_id,
                txid,
                vout_index,
                charm_id,
                block_height,
                data,
                asset_type,
                blockchain,
                network,
            )| {
                // [RJJ-SUPPLY] Extract supply from data JSON
                // NFTs start with supply=0 (tracks child tokens only)
                // Tokens use the supply value from the charm's amount
                let initial_supply = if asset_type == "nft" {
                    Decimal::ZERO
                } else {
                    // Extract supply from data.supply field
                    if let Some(supply_value) = data.get("supply") {
                        if let Some(supply_i64) = supply_value.as_i64() {
                            Decimal::from(supply_i64)
                        } else {
                            Decimal::from(1) // Fallback
                        }
                    } else {
                        Decimal::from(1) // Fallback if no supply in data
                    }
                };

                assets::ActiveModel {
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
                    name: NotSet,
                    symbol: NotSet,
                    description: NotSet,
                    image_url: NotSet,
                    total_supply: Set(Some(initial_supply)),
                    decimals: Set(DEFAULT_DECIMALS as i16), // [RJJ-DECIMALS] Default for batch
                    created_at: Set(now.into()),
                    updated_at: Set(now.into()),
                }
            },
        )
        .collect();

    // Try to insert all assets, handle duplicate key violations gracefully
    match Assets::insert_many(active_models).exec(db).await {
        Ok(_) => {
            // Only log for larger batches
            if assets_count > 5 {
                println!("✅ Batch saved {} assets successfully", assets_count);
            }
            Ok(())
        }
        Err(e) => {
            // Check if the error is a duplicate key violation
            if e.to_string()
                .contains("duplicate key value violates unique constraint")
            {
                // Assets already exist, this is not an error - silently succeed
                Ok(())
            } else {
                // If it's not a duplicate key error, propagate it
                Err(DbError::SeaOrmError(e))
            }
        }
    }
}
