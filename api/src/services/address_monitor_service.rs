use crate::db::repositories::{
    AddressTransactionsRepository, MonitoredAddressesRepository, UtxoRepository,
};
use crate::services::mempool_space_service::EsploraClient;

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
///    provider (Esplora / mempool.space) and registers it. From that moment on,
///    the indexer keeps the UTXO set up to date as new blocks arrive.
///
/// An advisory lock prevents concurrent seeding of the same address.
pub struct AddressMonitorService;

impl AddressMonitorService {
    /// Ensure an address is monitored and has UTXO + tx history data.
    /// If not yet monitored, seeds from Esplora (mempool.space) and registers.
    /// Returns true if the address was already monitored, false if freshly seeded.
    pub async fn ensure_monitored(
        monitored_repo: &MonitoredAddressesRepository,
        utxo_repo: &UtxoRepository,
        address_tx_repo: &AddressTransactionsRepository,
        esplora: &EsploraClient,
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
            // No age tracked but seeded → legacy row.
            return Ok(true);
        }

        // 2. Acquire advisory lock to prevent concurrent seeding
        let locked = monitored_repo.try_advisory_lock(address, network).await?;
        if !locked {
            // Another request is seeding this address — wait briefly and check again
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            return monitored_repo.is_seeded(address, network).await;
        }

        // 3. Double-check after acquiring lock (another request may have just
        //    refreshed). Only short-circuit when the seed is also FRESH —
        //    otherwise the lock + refresh path is the whole point.
        if let Some(age) = monitored_repo.seed_age_seconds(address, network).await.ok().flatten() {
            if age < MEMPOOL_REFRESH_SECS {
                let _ = monitored_repo.release_advisory_lock(address, network).await;
                return Ok(true);
            }
        }

        // 4. Seed UTXOs + tx history from Esplora
        let seed_result = Self::seed_from_esplora(
            utxo_repo, address_tx_repo, esplora, address, network,
        )
        .await;

        // 5. Capture chain tip (height + hash) — used as the hand-off cursor
        // so the indexer can validate continuity once it takes over.
        // `(0, None)` if the tip lookup fails.
        let (seed_height, seed_block_hash) = match esplora.get_chain_tip(network, false).await {
            Ok(tip) => (tip.height as i32, Some(tip.hash)),
            Err(e) => {
                tracing::warn!("Seed: tip lookup failed for {}: {}", network, e);
                (0, None)
            }
        };

        // 6. Register the address as monitored
        let _ = monitored_repo
            .register_seeded(address, network, seed_height, seed_block_hash.as_deref())
            .await;

        // 7. Release advisory lock
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

    /// Fetch UTXOs and tx history from Esplora and insert into DB tables.
    async fn seed_from_esplora(
        utxo_repo: &UtxoRepository,
        address_tx_repo: &AddressTransactionsRepository,
        esplora: &EsploraClient,
        address: &str,
        network: &str,
    ) -> Result<(usize, usize), String> {
        let (utxos, txs) = esplora.get_address_info(network, address).await?;

        // Insert UTXOs. block_height comes from Esplora's `status.block_height`;
        // 0 means mempool. `source` is "backfill" (allowed by the CHECK constraint).
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
                    source: "backfill".to_string(),
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
