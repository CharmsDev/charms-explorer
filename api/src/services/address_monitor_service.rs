use crate::db::repositories::{MonitoredAddressesRepository, UtxoRepository};
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
    /// Ensure an address is monitored and has UTXO data.
    /// If not yet monitored, seeds from QuickNode and registers.
    /// Returns true if the address was already monitored, false if freshly seeded.
    pub async fn ensure_monitored(
        monitored_repo: &MonitoredAddressesRepository,
        utxo_repo: &UtxoRepository,
        http_client: &reqwest::Client,
        quicknode_url: &str,
        address: &str,
        network: &str,
    ) -> Result<bool, String> {
        // 1. Check if already monitored
        if monitored_repo.is_monitored(address, network).await? {
            return Ok(true);
        }

        // 2. No QuickNode URL → cannot seed, skip silently
        if quicknode_url.is_empty() {
            return Ok(false);
        }

        // 3. Acquire advisory lock to prevent concurrent seeding
        let locked = monitored_repo.try_advisory_lock(address, network).await?;
        if !locked {
            // Another request is seeding this address — wait briefly and check again
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            return monitored_repo.is_monitored(address, network).await;
        }

        // 4. Double-check after acquiring lock (another request may have finished)
        if monitored_repo.is_monitored(address, network).await? {
            let _ = monitored_repo.release_advisory_lock(address, network).await;
            return Ok(true);
        }

        // 5. Seed UTXOs from QuickNode
        let seed_result =
            Self::seed_from_quicknode(utxo_repo, http_client, quicknode_url, address, network)
                .await;

        // 6. Get current chain height for seed_height
        let seed_height =
            match WalletService::get_chain_tip_quicknode(http_client, quicknode_url).await {
                Ok(tip) => tip.height as i32,
                Err(_) => 0,
            };

        // 7. Register the address as monitored
        let _ = monitored_repo
            .register_seeded(address, network, seed_height)
            .await;

        // 8. Release advisory lock
        let _ = monitored_repo.release_advisory_lock(address, network).await;

        match seed_result {
            Ok(count) => {
                tracing::info!(
                    "Seeded {} UTXOs for address {} (network: {}, height: {})",
                    count,
                    address,
                    network,
                    seed_height
                );
                Ok(false)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to seed UTXOs for {}: {} — address registered but may have incomplete data",
                    address,
                    e
                );
                Ok(false)
            }
        }
    }

    /// Fetch UTXOs from QuickNode and insert into address_utxos table.
    async fn seed_from_quicknode(
        utxo_repo: &UtxoRepository,
        http_client: &reqwest::Client,
        quicknode_url: &str,
        address: &str,
        network: &str,
    ) -> Result<usize, String> {
        let utxos = WalletService::get_utxos_quicknode(http_client, quicknode_url, address).await?;

        if utxos.is_empty() {
            return Ok(0);
        }

        // Convert to UtxoInsert format for batch insert
        let inserts: Vec<crate::db::repositories::utxo_repository::UtxoInsert> = utxos
            .iter()
            .map(|u| crate::db::repositories::utxo_repository::UtxoInsert {
                txid: u.txid.clone(),
                vout: u.vout as i32,
                address: address.to_string(),
                value: u.value as i64,
                script_pubkey: u.script_pubkey.clone(),
                block_height: 0, // QuickNode doesn't provide block height; indexer will update
                network: network.to_string(),
            })
            .collect();

        let count = inserts.len();
        utxo_repo.insert_batch(&inserts).await?;
        Ok(count)
    }
}
