pub mod address_extractor;
pub mod charm; // Modular charm service
pub mod dex; // DEX detection for Charms Cast
pub mod native_charm_parser;
pub mod tx_analyzer;

// Re-export services for direct imports
pub use address_extractor::AddressExtractor;
pub use charm::CharmService; // Now from the charm module
pub use native_charm_parser::{AssetInfo, NativeCharmParser};
