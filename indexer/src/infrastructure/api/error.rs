use std::error::Error;
use std::fmt;

/// Error type for API client operations
#[derive(Debug)]
pub enum ApiClientError {
    /// Error from the reqwest HTTP client
    HttpError(reqwest::Error),
    /// Error parsing JSON
    JsonError(serde_json::Error),
    /// API error
    ApiError(String),
    /// Response error
    ResponseError(String),
    /// Other error
    Other(String),
}

impl fmt::Display for ApiClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiClientError::HttpError(e) => write!(f, "HTTP error: {}", e),
            ApiClientError::JsonError(e) => write!(f, "JSON error: {}", e),
            ApiClientError::ApiError(msg) => write!(f, "API error: {}", msg),
            ApiClientError::ResponseError(msg) => write!(f, "Response error: {}", msg),
            ApiClientError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl Error for ApiClientError {}

impl From<reqwest::Error> for ApiClientError {
    fn from(error: reqwest::Error) -> Self {
        ApiClientError::HttpError(error)
    }
}

impl From<serde_json::Error> for ApiClientError {
    fn from(error: serde_json::Error) -> Self {
        ApiClientError::JsonError(error)
    }
}
