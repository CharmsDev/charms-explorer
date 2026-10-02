use std::error::Error;
use std::fmt;

/// Represents errors that can occur in Bitcoin client operations
#[derive(Debug)]
pub enum BitcoinClientError {
    /// Connection error
    ConnectionError(String),
    /// Configuration error
    ConfigError(String),
    /// Network error
    NetworkError(String),
    /// Parse error
    ParseError(String),
    /// Other error
    Other(String),
}

impl Clone for BitcoinClientError {
    fn clone(&self) -> Self {
        match self {
            BitcoinClientError::ConnectionError(msg) => BitcoinClientError::ConnectionError(msg.clone()),
            BitcoinClientError::ConfigError(msg) => BitcoinClientError::ConfigError(msg.clone()),
            BitcoinClientError::NetworkError(msg) => BitcoinClientError::NetworkError(msg.clone()),
            BitcoinClientError::ParseError(msg) => BitcoinClientError::ParseError(msg.clone()),
            BitcoinClientError::Other(msg) => BitcoinClientError::Other(msg.clone()),
        }
    }
}

impl fmt::Display for BitcoinClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BitcoinClientError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            BitcoinClientError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            BitcoinClientError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            BitcoinClientError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            BitcoinClientError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl Error for BitcoinClientError {}
