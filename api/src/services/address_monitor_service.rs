use crate::db::repositories::{
    AddressTransactionsRepository, MonitoredAddressesRepository, UtxoRepository,
};
use crate::services::maestro_service;
use crate::services::wallet_service::WalletService;

/// Service for on-demand address monitoring.
///
/// The `monitored_addresses` table starts empty. Addresses enter the system via:
///
/// 1. **Indexer (charm detection)** — Any address that receives a charm is
///    auto-registered during block processing. These addresses already have
///    their BTC UTXOs tracked by the indexer in real time.
///
/// 2. **API (this service)** — When a balance request arrives for an address
///    that is NOT yet monitored (e.g. a plain BTC address that has never held
///    a charm), this service seeds its current UTXO set from an external
///    provider (QuickNode / Mempool) and registers it. From that moment on,
///    the indexer keeps the UTXO set up to date as new blocks arrive.
///
/// An advisory lock prevents concurrent seeding of the same address.
pub struct AddressMonitorService;

impl AddressMonitorService {
    /// Ensure an address is monitored and has UTXO + tx history data.
    /// If not yet monitored, seeds from QuickNode (bb_getAddress) and registers.
    /// Returns true if the address was already monitored, false if freshly seeded.
    pub async fn ensure_monitored(
        monitored_repo: &MonitoredAddressesRepository,
        utxo_repo: &UtxoRepository,
        address_tx_repo: &AddressTransactionsRepository,
        http_client: &reqwest::Client,
        quicknode_url: &str,
        maestro_api_key: &str,
        address: &str,
        network: &str,
    ) -> Result<bool, String> {
        // 1. Soft refresh: a row marked `seeded` is still re-fetched from the
        //    provider when its last seed is older than MEMPOOL_REFRESH. The
        //    indexer only updates `address_utxos` at block confirmation, so a
        //    fresh mempool tx to a monitored address would otherwise stay
        //    invisible until the next block. Charm-bearing addresses still
        //    get the indexer's mempool processing for charms; this covers
        //    plain BTC UTXOs.
        const MEMPOOL_REFRESH_SECS: i64 = 15;
        let seed_age = monitored_repo.seed_age_seconds(address, network).await.ok().flatten();
        if let Some(age) = seed_age {
            if age < MEMPOOL_REFRESH_SECS {
                return Ok(true);
            }
            // Stale: fall through to re-seed.
        } else if monitored_repo.is_seeded(address, network).await? {
            // No age tracked but seeded → legacy / no providers configured.
            return Ok(true);
        }

        // 2. No provider URL → cannot seed, skip silently
        if quicknode_url.is_empty() && maestro_api_key.is_empty() {
            return Ok(false);
        }

        // 3. Acquire advisory lock to prevent concurrent seeding
        let locked = monitored_repo.try_advisory_lock(address, network).await?;
        if !locked {
            // Another request is seeding this address — wait briefly and check again
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            return monitored_repo.is_seeded(address, network).await;
        }

        // 4. Double-check after acquiring lock (another request may have just
        //    refreshed). Only short-circuit when the seed is also FRESH —
        //    otherwise the lock + refresh path is the whole point.
        if let Some(age) = monitored_repo.seed_age_seconds(address, network).await.ok().flatten() {
            if age < MEMPOOL_REFRESH_SECS {
                let _ = monitored_repo.release_advisory_lock(address, network).await;
                return Ok(true);
            }
        }

        // 5. Seed UTXOs + tx history: Maestro first → QuickNode fallback
        let seed_result = if !maestro_api_key.is_empty() {
            match Self::seed_from_maestro(
                utxo_repo, address_tx_repo, http_client, maestro_api_key, address, network,
            ).await {
                Ok(r) => Ok(r),
                Err(e) => {
                    tracing::warn!("Seed: Maestro failed for {}, trying QuickNode: {}", address, e);
                    if !quicknode_url.is_empty() {
                        Self::seed_from_quicknode(
                            utxo_repo, address_tx_repo, http_client, quicknode_url, address, network,
                        ).await
                    } else {
                        Err(e)
                    }
                }
            }
        } else {
            Self::seed_from_quicknode(
                utxo_repo, address_tx_repo, http_client, quicknode_url, address, network,
            ).await
        };

        // 6. Capture chain tip (height + hash) — used as the hand-off cursor
        // so the indexer can validate continuity once it takes over.
        let (seed_height, seed_block_hash) = Self::capture_tip(
            http_client,
            maestro_api_key,
            quicknode_url,
            network,
        )
        .await;

        // 7. Register the address as monitored
        let _ = monitored_repo
            .register_seeded(address, network, seed_height, seed_block_hash.as_deref())
            .await;

        // 8. Release advisory lock
        let _ = monitored_repo.release_advisory_lock(address, network).await;

        match seed_result {
            Ok((utxo_count, tx_count)) => {
                tracing::info!(
                    "Seeded {} UTXOs + {} txs for address {} (network: {}, height: {})",
                    utxo_count,
                    tx_count,
                    address,
                    network,
                    seed_height
                );
                Ok(false)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to seed address {}: {} — registered but may have incomplete data",
                    address,
                    e
                );
                Ok(false)
            }
        }
    }

    /// Capture chain tip from Maestro (preferred) → QuickNode (fallback).
    /// Returns `(height, Some(hash))` on success, `(0, None)` if both fail.
    /// The hash is the cursor used by the indexer to validate handoff
    /// continuity — without it we cannot detect a reorg that happened
    /// between the seed and the first indexed block.
    async fn capture_tip(
        http_client: &reqwest::Client,
        maestro_api_key: &str,
        quicknode_url: &str,
        network: &str,
    ) -> (i32, Option<String>) {
        if !maestro_api_key.is_empty() {
            if let Ok(tip) = maestro_service::get_chain_tip(http_client, maestro_api_key, network).await {
                return (tip.height as i32, Some(tip.hash));
            }
        }
        if !quicknode_url.is_empty() {
            if let Ok(tip) = WalletService::get_chain_tip_quicknode(http_client, quicknode_url).await
            {
                return (tip.height as i32, Some(tip.hash));
            }
        }
        (0, None)
    }

    /// Fetch UTXOs and tx history from Maestro and insert into DB tables.
    async fn seed_from_maestro(
        utxo_repo: &UtxoRepository,
        address_tx_repo: &AddressTransactionsRepository,
        http_client: &reqwest::Client,
        maestro_api_key: &str,
        address: &str,
        network: &str,
    ) -> Result<(usize, usize), String> {
        let (utxos, txs) =
            maestro_service::get_address_info(http_client, maestro_api_key, network, address).await?;

        // Insert UTXOs. block_height comes from the provider's `status.block_height`
        // (or `height` for indexed responses); 0 means mempool.
        let utxo_count = if !utxos.is_empty() {
            let inserts: Vec<crate::db::repositories::utxo_repository::UtxoInsert> = utxos
                .iter()
                .map(|u| crate::db::repositories::utxo_repository::UtxoInsert {
                    txid: u.txid.clone(),
                    vout: u.vout as i32,
                    address: address.to_string(),
                    value: u.value as i64,
                    script_pubkey: u.script_pubkey.clone(),
                    block_height: u.block_height.map(|h| h as i32).unwrap_or(0),
                    network: network.to_string(),
                    source: "maestro".to_string(),
                })
                .collect();
            let count = inserts.len();
            utxo_repo.insert_batch(&inserts).await?;
            count
        } else {
            0
        };

        // Insert transaction history
        let tx_count = if !txs.is_empty() {
            let tx_inserts: Vec<
                crate::db::repositories::address_transactions_repository::AddressTxInsert,
            > = txs
                .iter()
                .map(|t| {
                    crate::db::repositories::address_transactions_repository::AddressTxInsert {
                        txid: t.txid.clone(),
                        address: address.to_string(),
                        network: network.to_string(),
                        direction: t.direction.clone(),
                        amount: t.amount,
                        fee: t.fee,
                        block_height: t.block_height,
                        block_time: t.block_time,
                        confirmations: t.confirmations,
                    }
                })
                .collect();
            let count = tx_inserts.len();
            address_tx_repo.insert_batch(&tx_inserts).await?;
            count
        } else {
            0
        };

        Ok((utxo_count, tx_count))
    }

    /// Fetch UTXOs and tx history from QuickNode and insert into DB tables.
    async fn seed_from_quicknode(
        utxo_repo: &UtxoRepository,
        address_tx_repo: &AddressTransactionsRepository,
        http_client: &reqwest::Client,
        quicknode_url: &str,
        address: &str,
        network: &str,
    ) -> Result<(usize, usize), String> {
        let (utxos, txs) =
            WalletService::get_address_quicknode(http_client, quicknode_url, address).await?;

        // Insert UTXOs. block_height comes from the provider's `status.block_height`
        // (or `height` for indexed responses); 0 means mempool.
        let utxo_count = if !utxos.is_empty() {
            let inserts: Vec<crate::db::repositories::utxo_repository::UtxoInsert> = utxos
                .iter()
                .map(|u| crate::db::repositories::utxo_repository::UtxoInsert {
                    txid: u.txid.clone(),
                    vout: u.vout as i32,
                    address: address.to_string(),
                    value: u.value as i64,
                    script_pubkey: u.script_pubkey.clone(),
                    block_height: u.block_height.map(|h| h as i32).unwrap_or(0),
                    network: network.to_string(),
                    source: "maestro".to_string(),
                })
                .collect();
            let count = inserts.len();
            utxo_repo.insert_batch(&inserts).await?;
            count
        } else {
            0
        };

        // Insert transaction history
        let tx_count = if !txs.is_empty() {
            let tx_inserts: Vec<
                crate::db::repositories::address_transactions_repository::AddressTxInsert,
            > = txs
                .iter()
                .map(|t| {
                    crate::db::repositories::address_transactions_repository::AddressTxInsert {
                        txid: t.txid.clone(),
                        address: address.to_string(),
                        network: network.to_string(),
                        direction: t.direction.clone(),
                        amount: t.amount,
                        fee: t.fee,
                        block_height: t.block_height,
                        block_time: t.block_time,
                        confirmations: t.confirmations,
                    }
                })
                .collect();
            let count = tx_inserts.len();
            address_tx_repo.insert_batch(&tx_inserts).await?;
            count
        } else {
            0
        };

        Ok((utxo_count, tx_count))
    }
}
