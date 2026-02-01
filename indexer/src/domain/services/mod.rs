pub mod address_extractor;
pub mod asset_supply_calculator;
pub mod charm; // Modular charm service
pub mod charm_queue_service;
pub mod dex; // DEX detection for Charms Cast
pub mod native_charm_parser;
pub mod reference_nft_cache;

// Re-export services for direct imports
pub use address_extractor::AddressExtractor;
pub use asset_supply_calculator::AssetSupplyCalculator;
pub use charm::CharmService; // Now from the charm module
pub use charm_queue_service::CharmQueueService;
pub use native_charm_parser::{AssetInfo, NativeCharmParser};
pub use reference_nft_cache::{ReferenceNftCache, ReferenceNftMetadata};
