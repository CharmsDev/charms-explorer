use std::error::Error;
use std::fmt;

/// Represents errors that can occur in Bitcoin client operations
#[derive(Debug)]
pub enum BitcoinClientError {
    /// Error from the Bitcoin Core RPC client
    RpcError(bitcoincore_rpc::Error),
    /// Connection error
    ConnectionError(String),
    /// Configuration error
    ConfigError(String),
    /// Other error
    Other(String),
}

impl Clone for BitcoinClientError {
    fn clone(&self) -> Self {
        match self {
            BitcoinClientError::RpcError(e) => BitcoinClientError::Other(e.to_string()),
            BitcoinClientError::ConnectionError(msg) => BitcoinClientError::ConnectionError(msg.clone()),
            BitcoinClientError::ConfigError(msg) => BitcoinClientError::ConfigError(msg.clone()),
            BitcoinClientError::Other(msg) => BitcoinClientError::Other(msg.clone()),
        }
    }
}

impl fmt::Display for BitcoinClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BitcoinClientError::RpcError(e) => write!(f, "Bitcoin RPC error: {}", e),
            BitcoinClientError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            BitcoinClientError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            BitcoinClientError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl Error for BitcoinClientError {}

impl From<bitcoincore_rpc::Error> for BitcoinClientError {
    fn from(error: bitcoincore_rpc::Error) -> Self {
        BitcoinClientError::RpcError(error)
    }
}
