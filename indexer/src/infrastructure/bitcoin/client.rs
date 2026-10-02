use bitcoincore_rpc::bitcoin::{Block, BlockHash};
use std::str::FromStr;
use std::sync::Arc;

use crate::config::{BitcoinConfig, NetworkId, NetworkType};
use crate::infrastructure::bitcoin::error::BitcoinClientError;
use crate::infrastructure::bitcoin::esplora::EsploraClient;
use crate::infrastructure::bitcoin::mempool_stream::MempoolStream;
use crate::utils::logging;

/// Chain data access for one network, backed by a public Esplora gateway
/// (mempool.space) plus its websocket mempool feed.
#[derive(Debug, Clone)]
pub struct BitcoinClient {
    esplora: EsploraClient,
    ws_url: String,
    network_id: NetworkId,
}

impl BitcoinClient {
    pub fn new(bitcoin_config: &BitcoinConfig) -> Result<Self, BitcoinClientError> {
        let network_id = NetworkId::new(NetworkType::Bitcoin, &bitcoin_config.network);
        logging::log_info(&format!(
            "[{}] Esplora gateway: {}",
            network_id.name, bitcoin_config.esplora_url
        ));
        Ok(Self {
            esplora: EsploraClient::new(&bitcoin_config.esplora_url),
            ws_url: bitcoin_config.ws_url.clone(),
            network_id,
        })
    }

    pub fn network_id(&self) -> &NetworkId {
        &self.network_id
    }

    pub fn esplora(&self) -> &EsploraClient {
        &self.esplora
    }

    /// Start the websocket mempool feed for this network.
    pub fn spawn_mempool_stream(&self) -> Arc<MempoolStream> {
        MempoolStream::spawn(self.network_id.name.clone(), self.ws_url.clone())
    }

    /// Current tip height.
    pub async fn get_block_count(&self) -> Result<u64, BitcoinClientError> {
        self.esplora.tip_height().await
    }

    pub async fn get_block_hash(&self, height: u64) -> Result<BlockHash, BitcoinClientError> {
        parse_hash(&self.esplora.block_hash(height).await?)
    }

    pub async fn get_best_block_hash(&self) -> Result<BlockHash, BitcoinClientError> {
        parse_hash(&self.esplora.tip_hash().await?)
    }

    pub async fn get_block(&self, hash: &BlockHash) -> Result<Block, BitcoinClientError> {
        let bytes = self.esplora.block_raw(&hash.to_string()).await?;
        bitcoincore_rpc::bitcoin::consensus::deserialize(&bytes)
            .map_err(|e| BitcoinClientError::ParseError(format!("block {}: {}", hash, e)))
    }

    /// Raw tx hex. The gateway has a full tx index, so no block hash needed.
    pub async fn get_raw_transaction_hex(&self, txid: &str) -> Result<String, BitcoinClientError> {
        self.esplora.tx_hex(txid).await
    }
}

fn parse_hash(s: &str) -> Result<BlockHash, BitcoinClientError> {
    BlockHash::from_str(s.trim())
        .map_err(|e| BitcoinClientError::ParseError(format!("block hash {:?}: {}", s, e)))
}

#[cfg(test)]
mod live_tests {
    use super::*;

    fn client(network: &str) -> BitcoinClient {
        use crate::infrastructure::bitcoin::esplora::{default_esplora_url, default_ws_url};
        BitcoinClient::new(&BitcoinConfig {
            network: network.to_string(),
            genesis_block_height: 0,
            esplora_url: default_esplora_url(network),
            ws_url: default_ws_url(network),
        })
        .unwrap()
    }

    /// Hits mempool.space: `cargo test -- --ignored live_`
    #[tokio::test]
    #[ignore]
    async fn live_fetches_and_parses_tip_block() {
        for net in ["mainnet", "testnet4"] {
            let c = client(net);
            let tip = c.get_block_count().await.unwrap();
            let hash = c.get_block_hash(tip - 1).await.unwrap();
            let block = c.get_block(&hash).await.unwrap();
            assert_eq!(block.block_hash(), hash);
            let tx = &block.txdata[block.txdata.len() - 1];
            let hex = c.get_raw_transaction_hex(&tx.txid().to_string()).await.unwrap();
            assert_eq!(hex, hex::encode(bitcoincore_rpc::bitcoin::consensus::serialize(tx)));
            println!("{} tip={} txs={}", net, tip, block.txdata.len());
        }
    }

    #[tokio::test]
    #[ignore]
    async fn live_mempool_stream_rebuilds_txs() {
        let stream = client("mainnet").spawn_mempool_stream();
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        let txs = stream.drain(usize::MAX).await;
        let rebuilt = txs.iter().filter(|t| t.hex.is_some()).count();
        println!("streamed={} rebuilt={}", txs.len(), rebuilt);
        assert!(!txs.is_empty());
        assert!(rebuilt * 100 >= txs.len() * 99);
    }
}
