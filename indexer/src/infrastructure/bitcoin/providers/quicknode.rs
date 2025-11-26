//! QuickNode provider implementation

use async_trait::async_trait;
use bitcoincore_rpc::bitcoin::{Block, BlockHash};
use reqwest::Client;
use serde_json::{json, Value};
use std::str::FromStr;

use crate::infrastructure::bitcoin::error::BitcoinClientError;
use super::BitcoinProvider;

/// QuickNode provider for Bitcoin RPC calls
#[derive(Debug)]
pub struct QuickNodeProvider {
    endpoint: String,
    client: Client,
}

impl QuickNodeProvider {
    /// Create a new QuickNode provider
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            client: Client::new(),
        }
    }

    /// Make a JSON-RPC call to QuickNode
    async fn rpc_call(&self, method: &str, params: Value) -> Result<Value, BitcoinClientError> {
        let request_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| BitcoinClientError::NetworkError(e.to_string()))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| BitcoinClientError::NetworkError(e.to_string()))?;

        let response_json: Value = serde_json::from_str(&response_text)
            .map_err(|e| BitcoinClientError::ParseError(e.to_string()))?;

        if let Some(error) = response_json.get("error") {
            return Err(BitcoinClientError::NetworkError(error.to_string()));
        }

        response_json
            .get("result")
            .cloned()
            .ok_or_else(|| BitcoinClientError::ParseError("No result in response".to_string()))
    }
}

#[async_trait]
impl BitcoinProvider for QuickNodeProvider {
    fn provider_name(&self) -> String {
        "QuickNode".to_string()
    }

    async fn get_block_count(&self) -> Result<u64, BitcoinClientError> {
        let result = self.rpc_call("getblockcount", json!([])).await?;
        result
            .as_u64()
            .ok_or_else(|| BitcoinClientError::ParseError("Invalid block count".to_string()))
    }

    async fn get_block_hash(&self, height: u64) -> Result<BlockHash, BitcoinClientError> {
        let result = self.rpc_call("getblockhash", json!([height])).await?;
        let hash_str = result
            .as_str()
            .ok_or_else(|| BitcoinClientError::ParseError("Invalid block hash".to_string()))?;
        
        BlockHash::from_str(hash_str)
            .map_err(|e| BitcoinClientError::ParseError(e.to_string()))
    }

    async fn get_block(&self, block_hash: &BlockHash) -> Result<Block, BitcoinClientError> {
        // Get raw block data as hex string
        let result = self
            .rpc_call("getblock", json!([block_hash.to_string(), 0]))
            .await?;
        
        let block_hex = result
            .as_str()
            .ok_or_else(|| BitcoinClientError::ParseError("Invalid block hex".to_string()))?;
        
        let block_bytes = hex::decode(block_hex)
            .map_err(|e| BitcoinClientError::ParseError(e.to_string()))?;
        
        bitcoincore_rpc::bitcoin::consensus::deserialize(&block_bytes)
            .map_err(|e| BitcoinClientError::ParseError(e.to_string()))
    }

    async fn get_raw_transaction_hex(
        &self,
        txid: &str,
        _block_hash: Option<&BlockHash>,
    ) -> Result<String, BitcoinClientError> {
        let result = self
            .rpc_call("getrawtransaction", json!([txid, false]))
            .await?;
        
        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| BitcoinClientError::ParseError("Invalid transaction hex".to_string()))
    }

    async fn apply_rate_limiting(&self) {
        // No rate limiting for maximum performance
    }
}
