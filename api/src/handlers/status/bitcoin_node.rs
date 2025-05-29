// Bitcoin node information module for status handler

use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;

/// Gets Bitcoin node information by directly connecting to the Bitcoin RPC
pub async fn get_bitcoin_node_info(
    _host: &str,
    _port: &str,
    _username: &str,
    _password: &str,
    network_type: &str,
) -> Value {
    // Get configuration
    let config = crate::config::ApiConfig::from_env();

    // Use the appropriate RPC connection based on the network type
    if network_type == "mainnet" {
        // For mainnet, use the mainnet-specific RPC settings
        let rpc_url = format!(
            "http://{}:{}",
            config.bitcoin_mainnet_rpc_host, config.bitcoin_mainnet_rpc_port
        );
        let auth = Auth::UserPass(
            config.bitcoin_mainnet_rpc_username.to_string(),
            config.bitcoin_mainnet_rpc_password.to_string(),
        );

        // Try to connect to the Bitcoin RPC server
        match Client::new(&rpc_url, auth) {
            Ok(client) => {
                // Try to get the block count with a timeout to prevent hanging
                let block_count_result =
                    timeout(Duration::from_secs(5), async { client.get_block_count() }).await;

                match block_count_result {
                    Ok(Ok(block_count)) => {
                        // If block count succeeded, try to get the best block hash
                        let best_block_hash = match client.get_best_block_hash() {
                            Ok(hash) => hash.to_string(),
                            Err(_) => "unknown".to_string(),
                        };

                        // Use the provided network type
                        json!({
                            "status": "connected",
                            "network": network_type,
                            "block_count": block_count,
                            "best_block_hash": best_block_hash
                        })
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Failed to get block count for mainnet: {}", e);
                        json!({
                            "status": "error",
                            "network": network_type,
                            "block_count": 0,
                            "best_block_hash": "unknown",
                            "error": format!("Failed to connect to Bitcoin mainnet node: {}", e)
                        })
                    }
                    Err(_) => {
                        tracing::error!("Bitcoin mainnet RPC request timed out after 5 seconds");
                        tracing::error!(
                            "RPC URL: {}, Username: {}",
                            rpc_url,
                            config.bitcoin_mainnet_rpc_username
                        );
                        json!({
                            "status": "timeout",
                            "network": network_type,
                            "block_count": 0,
                            "best_block_hash": "unknown",
                            "error": "RPC request timed out. Check if the Bitcoin node is accessible from this environment."
                        })
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to connect to Bitcoin mainnet RPC: {}", e);
                tracing::error!(
                    "RPC URL: {}, Username: {}",
                    rpc_url,
                    config.bitcoin_mainnet_rpc_username
                );
                json!({
                    "status": "error",
                    "network": network_type,
                    "block_count": 0,
                    "best_block_hash": "unknown",
                    "error": format!("Connection error: {}. Check if the Bitcoin node is accessible from this environment.", e)
                })
            }
        }
    } else {
        // For testnet4, use the testnet4-specific RPC settings
        let rpc_url = format!(
            "http://{}:{}",
            config.bitcoin_testnet4_rpc_host, config.bitcoin_testnet4_rpc_port
        );
        let auth = Auth::UserPass(
            config.bitcoin_testnet4_rpc_username.to_string(),
            config.bitcoin_testnet4_rpc_password.to_string(),
        );

        // Try to connect to the Bitcoin RPC server
        match Client::new(&rpc_url, auth) {
            Ok(client) => {
                // Try to get the block count with a timeout to prevent hanging
                let block_count_result =
                    timeout(Duration::from_secs(5), async { client.get_block_count() }).await;

                match block_count_result {
                    Ok(Ok(block_count)) => {
                        // If block count succeeded, try to get the best block hash
                        let best_block_hash = match client.get_best_block_hash() {
                            Ok(hash) => hash.to_string(),
                            Err(_) => "unknown".to_string(),
                        };

                        // Use the provided network type
                        json!({
                            "status": "connected",
                            "network": network_type,
                            "block_count": block_count,
                            "best_block_hash": best_block_hash
                        })
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Failed to get block count for testnet4: {}", e);
                        json!({
                            "status": "error",
                            "network": network_type,
                            "block_count": 0,
                            "best_block_hash": "unknown"
                        })
                    }
                    Err(_) => {
                        tracing::error!("Bitcoin testnet4 RPC request timed out after 5 seconds");
                        tracing::error!(
                            "RPC URL: {}, Username: {}",
                            rpc_url,
                            config.bitcoin_testnet4_rpc_username
                        );
                        json!({
                            "status": "timeout",
                            "network": network_type,
                            "block_count": 0,
                            "best_block_hash": "unknown",
                            "error": "RPC request timed out. Check if the Bitcoin node is accessible from this environment."
                        })
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to connect to Bitcoin testnet4 RPC: {}", e);
                tracing::error!(
                    "RPC URL: {}, Username: {}",
                    rpc_url,
                    config.bitcoin_testnet4_rpc_username
                );
                json!({
                    "status": "error",
                    "network": network_type,
                    "block_count": 0,
                    "best_block_hash": "unknown",
                    "error": format!("Connection error: {}. Check if the Bitcoin node is accessible from this environment.", e)
                })
            }
        }
    }
}
