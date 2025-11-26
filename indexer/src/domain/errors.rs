use std::error::Error;
use std::fmt;

use crate::infrastructure::bitcoin::BitcoinClientError;
use crate::infrastructure::persistence::error::DbError;

/// Error type for charm detection and processing
#[derive(Debug)]
pub enum CharmError {
    BitcoinClientError(BitcoinClientError),
    DbError(DbError),
    DetectionError(String),
    ProcessingError(String),
}

impl fmt::Display for CharmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CharmError::BitcoinClientError(e) => write!(f, "Bitcoin client error: {}", e),
            CharmError::DbError(e) => write!(f, "Database error: {}", e),
            CharmError::DetectionError(msg) => write!(f, "Charm detection error: {}", msg),
            CharmError::ProcessingError(msg) => write!(f, "Charm processing error: {}", msg),
        }
    }
}

impl Error for CharmError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CharmError::BitcoinClientError(e) => Some(e),
            CharmError::DbError(e) => Some(e),
            CharmError::DetectionError(_) => None,
            CharmError::ProcessingError(_) => None,
        }
    }
}

impl From<BitcoinClientError> for CharmError {
    fn from(error: BitcoinClientError) -> Self {
        CharmError::BitcoinClientError(error)
    }
}

impl From<DbError> for CharmError {
    fn from(error: DbError) -> Self {
        CharmError::DbError(error)
    }
}

/// Error type for block processing operations
#[derive(Debug)]
pub enum BlockProcessorError {
    BitcoinClientError(BitcoinClientError),
    CharmError(CharmError),
    DbError(DbError),
    ConfigError(String),
    ProcessingError(String),
}

impl fmt::Display for BlockProcessorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockProcessorError::BitcoinClientError(e) => write!(f, "Bitcoin client error: {}", e),
            BlockProcessorError::CharmError(e) => write!(f, "Charm error: {}", e),
            BlockProcessorError::DbError(e) => write!(f, "Database error: {}", e),
            BlockProcessorError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            BlockProcessorError::ProcessingError(msg) => write!(f, "Processing error: {}", msg),
        }
    }
}

impl Error for BlockProcessorError {}

impl From<BitcoinClientError> for BlockProcessorError {
    fn from(error: BitcoinClientError) -> Self {
        BlockProcessorError::BitcoinClientError(error)
    }
}

impl From<CharmError> for BlockProcessorError {
    fn from(error: CharmError) -> Self {
        BlockProcessorError::CharmError(error)
    }
}

impl From<DbError> for BlockProcessorError {
    fn from(error: DbError) -> Self {
        BlockProcessorError::DbError(error)
    }
}
