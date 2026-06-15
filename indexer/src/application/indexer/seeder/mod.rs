//! BTC auto-seeder: proactively populates `address_utxos` and
//! `address_transactions` for every monitored address (typically a
//! charm-holder auto-registered by the block processor) by calling Maestro
//! once per address.
//!
//! Why this exists:
//! - The block processor adds rows to `monitored_addresses` the first
//!   time an address receives a charm. From then on the indexer maintains
//!   the address's BTC UTXO state forward via `utxo_indexer` and
//!   `mempool/utxo_tracker`. **But it does NOT know the BTC the address
//!   already had before charms first touched it** — that history lives
//!   pre-genesis (block < 895,206) and isn't replayed by the indexer.
//! - Before this worker, that historical BTC was only fetched when an
//!   end user hit the API and `AddressMonitorService::ensure_monitored`
//!   triggered an on-demand Maestro seed. Charm holders who never got
//!   queried had a `seeded_at = NULL` row and no historical UTXOs.
//! - This worker closes that gap proactively: it picks up unseeded
//!   monitored addresses in the background and seeds them via Maestro,
//!   so wallets see correct BTC balance the moment they first ask.
//!
//! Design constraints:
//! - Inocuous to indexing — runs as an independent supervised task, never
//!   blocks the block processor. Maestro outages just stall the queue.
//! - Idempotent — uses the same per-address advisory lock as the API
//!   on-demand seeder, so the two paths can't race on the same address.
//! - Rate limited — bounded concurrent in-flight requests; respects
//!   Maestro's quota.
//! - Cancellation aware — honours the shared `CancellationToken` for
//!   graceful shutdown.

pub mod worker;

pub use worker::{AddressSeeder, SeederConfig};
