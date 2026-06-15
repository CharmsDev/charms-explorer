//! Minimal Maestro HTTP client used by the BTC auto-seeder.
//!
//! Scope: just what the seeder needs — esplora `/address/{a}/utxo`,
//! paginated `/address/{a}/txs`, and `/blocks/tip/{height,hash}`. We
//! intentionally do NOT port the API's full client (broadcast, fee
//! estimates, indexed pagination for >1000 UTXO addresses): the seeder
//! processes one charm-holder address at a time and the >1000-UTXO case
//! is vanishingly rare for that population.

pub mod client;

pub use client::{
    MaestroClient, MaestroAddressTx, MaestroChainTip, MaestroError, MaestroUtxo,
};
