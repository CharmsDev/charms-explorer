use std::error::Error;
use std::fmt;

/// Error type for Bitcoin client operations
#[derive(Debug)]
pub enum BitcoinClientError {
    /// Error from the Bitcoin Core RPC client
    RpcError(bitcoincore_rpc::Error),
    /// Connection error
    ConnectionError(String),
    /// Other error
    Other(String),
}

impl fmt::Display for BitcoinClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BitcoinClientError::RpcError(e) => write!(f, "Bitcoin RPC error: {}", e),
            BitcoinClientError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
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
