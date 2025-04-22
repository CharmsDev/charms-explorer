// Error types for database operations

use thiserror::Error;

/// Error types for database connection and query operations
#[derive(Debug, Error)]
pub enum DbError {
    /// Error occurred during database connection attempt
    #[error("Database connection error: {0}")]
    ConnectionError(String),

    /// Error occurred during database query execution
    #[error("Database query error: {0}")]
    QueryError(String),
}

impl From<sea_orm::DbErr> for DbError {
    fn from(err: sea_orm::DbErr) -> Self {
        DbError::QueryError(err.to_string())
    }
}
