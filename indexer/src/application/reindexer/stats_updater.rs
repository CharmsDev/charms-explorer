//! Stats holders update logic for reindexer

use crate::infrastructure::persistence::repositories::{CharmRepository, StatsHoldersRepository};
use crate::utils::logging;
use super::types::{HolderUpdate, to_nft_app_id};

/// Update stats_holders for spent charms (reduce balances)
pub async fn update_spent_holders(
    charm_repository: &CharmRepository,
    stats_holders_repository: &StatsHoldersRepository,
    spent_txids: Vec<String>,
    block_height: u64,
    network: &str,
) {
    if spent_txids.is_empty() {
        return;
    }

    // Get charm info before marking as spent
    let charm_info = charm_repository
        .get_charms_for_spent_update(spent_txids.clone())
        .await
        .unwrap_or_default();

    // Mark as spent
    if let Err(e) = charm_repository.mark_charms_as_spent_batch(spent_txids).await {
        logging::log_warning(&format!(
            "[{}] Block {}: Mark spent error: {}",
            network, block_height, e
        ));
    }

    // Update stats_holders with negative amounts
    if !charm_info.is_empty() {
        let spent_holder_updates: Vec<HolderUpdate> = charm_info
            .into_iter()
            .map(|(app_id, address, amount)| {
                (to_nft_app_id(app_id), address, -amount, block_height as i32)
            })
            .collect();

        if let Err(e) = stats_holders_repository
            .update_holders_batch(spent_holder_updates)
            .await
        {
            logging::log_warning(&format!(
                "[{}] Block {}: Stats holders (spent) error: {}",
                network, block_height, e
            ));
        }
    }
}

/// Update stats_holders for new unspent charms (add balances)
pub async fn update_new_holders(
    charm_repository: &CharmRepository,
    stats_holders_repository: &StatsHoldersRepository,
    block_height: u64,
    network: &str,
) {
    let new_charms_for_stats: Vec<HolderUpdate> = charm_repository
        .get_unspent_charms_by_block(block_height as i32, network)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(app_id, address_opt, amount)| {
            address_opt.map(|address| {
                (to_nft_app_id(app_id), address, amount, block_height as i32)
            })
        })
        .collect();

    if !new_charms_for_stats.is_empty() {
        if let Err(e) = stats_holders_repository
            .update_holders_batch(new_charms_for_stats)
            .await
        {
            logging::log_warning(&format!(
                "[{}] Block {}: Stats holders (new) error: {}",
                network, block_height, e
            ));
        }
    }
}
