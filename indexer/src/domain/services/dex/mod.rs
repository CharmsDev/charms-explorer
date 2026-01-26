//! DEX module for Charms Cast order detection and parsing
//!
//! This module provides functionality to detect and parse DEX operations
//! from Charms transactions, specifically for the Charms Cast DEX.

pub mod detection;
pub mod types;

pub use detection::detect_dex_operation;
pub use types::{
    DexDetectionResult, DexOperation, DexOrder, ExecType, OrderSide, is_bro_token, is_dex_app_id,
};
