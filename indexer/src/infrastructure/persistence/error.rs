use std::error::Error;
use std::fmt;

/// Error type for database operations
#[derive(Debug)]
pub enum DbError {
    /// Error from SeaORM
    SeaOrmError(sea_orm::DbErr),
    /// Connection error
    ConnectionError(String),
    /// Query error
    QueryError(String),
    /// Other error
    Other(String),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::SeaOrmError(e) => write!(f, "Database error: {}", e),
            DbError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            DbError::QueryError(msg) => write!(f, "Query error: {}", msg),
            DbError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl Error for DbError {}

impl From<sea_orm::DbErr> for DbError {
    fn from(err: sea_orm::DbErr) -> Self {
        DbError::SeaOrmError(err)
    }
}
