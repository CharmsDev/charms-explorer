//! Reference NFT Cache
//! 
//! In-memory cache for reference NFT metadata to avoid repeated database lookups
//! and to efficiently mark NFTs as reference when tokens are found.
//!
//! Architecture:
//! - NFTs are cached by their hash (the part after n/ or t/ prefix)
//! - When a token is found, we lookup the parent NFT by hash
//! - If found in cache, we use cached metadata and skip DB lookup
//! - We also track which NFTs have been marked as reference to avoid repeated updates

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

/// Metadata extracted from reference NFT for token inheritance
#[derive(Debug, Clone)]
pub struct ReferenceNftMetadata {
    pub app_id: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub description: Option<String>,
    pub decimals: i16,
    // Note: image_url is NOT cached here - tokens should fetch it on demand
}

/// Thread-safe cache for reference NFT metadata
pub struct ReferenceNftCache {
    /// Map from hash -> NFT metadata
    /// Hash is extracted from app_id: n/HASH/TXID:VOUT -> HASH
    metadata_cache: RwLock<HashMap<String, ReferenceNftMetadata>>,
    
    /// Set of hashes that have already been marked as reference in DB
    /// Prevents repeated UPDATE queries for the same NFT
    marked_as_reference: RwLock<HashSet<String>>,
}

impl ReferenceNftCache {
    pub fn new() -> Self {
        Self {
            metadata_cache: RwLock::new(HashMap::new()),
            marked_as_reference: RwLock::new(HashSet::new()),
        }
    }

    /// Extract hash from app_id (works for both n/ and t/ prefixes)
    /// Example: n/3d7fe7e4.../txid:0 -> 3d7fe7e4...
    pub fn extract_hash(app_id: &str) -> Option<String> {
        if app_id.len() < 3 {
            return None;
        }
        
        // Skip prefix (n/ or t/)
        let without_prefix = &app_id[2..];
        
        // Hash is the part before the first /
        if let Some(slash_pos) = without_prefix.find('/') {
            Some(without_prefix[..slash_pos].to_string())
        } else {
            // No second slash, entire remainder is hash
            Some(without_prefix.to_string())
        }
    }

    /// Cache NFT metadata for later token lookups
    pub fn cache_nft(&self, app_id: &str, metadata: ReferenceNftMetadata) {
        if let Some(hash) = Self::extract_hash(app_id) {
            if let Ok(mut cache) = self.metadata_cache.write() {
                cache.insert(hash, metadata);
            }
        }
    }

    /// Get cached NFT metadata by hash
    pub fn get_by_hash(&self, hash: &str) -> Option<ReferenceNftMetadata> {
        if let Ok(cache) = self.metadata_cache.read() {
            cache.get(hash).cloned()
        } else {
            None
        }
    }

    /// Get cached NFT metadata for a token app_id
    pub fn get_for_token(&self, token_app_id: &str) -> Option<ReferenceNftMetadata> {
        Self::extract_hash(token_app_id).and_then(|hash| self.get_by_hash(&hash))
    }

    /// Check if NFT has already been marked as reference in DB
    pub fn is_marked_as_reference(&self, hash: &str) -> bool {
        if let Ok(set) = self.marked_as_reference.read() {
            set.contains(hash)
        } else {
            false
        }
    }

    /// Mark NFT as reference (record that we've updated the DB)
    pub fn mark_as_reference(&self, hash: &str) {
        if let Ok(mut set) = self.marked_as_reference.write() {
            set.insert(hash.to_string());
        }
    }

    /// Check if we need to mark this NFT as reference
    /// Returns true if this is the first time we're seeing a token for this NFT
    pub fn should_mark_as_reference(&self, token_app_id: &str) -> bool {
        if let Some(hash) = Self::extract_hash(token_app_id) {
            !self.is_marked_as_reference(&hash)
        } else {
            false
        }
    }

    /// Get cache statistics for logging
    pub fn stats(&self) -> (usize, usize) {
        let metadata_count = self.metadata_cache.read().map(|c| c.len()).unwrap_or(0);
        let marked_count = self.marked_as_reference.read().map(|s| s.len()).unwrap_or(0);
        (metadata_count, marked_count)
    }

    /// Clear the cache (useful for testing or reindexing)
    pub fn clear(&self) {
        if let Ok(mut cache) = self.metadata_cache.write() {
            cache.clear();
        }
        if let Ok(mut set) = self.marked_as_reference.write() {
            set.clear();
        }
    }
}

impl Default for ReferenceNftCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_hash() {
        assert_eq!(
            ReferenceNftCache::extract_hash("n/3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b/c975d4e0c292fb95efbda5c13312d6ac1d8b5aeff7f0f1e5578645a2da70ff5f:0"),
            Some("3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b".to_string())
        );
        
        assert_eq!(
            ReferenceNftCache::extract_hash("t/3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b/c975d4e0c292fb95efbda5c13312d6ac1d8b5aeff7f0f1e5578645a2da70ff5f:0"),
            Some("3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b".to_string())
        );
    }

    #[test]
    fn test_cache_and_retrieve() {
        let cache = ReferenceNftCache::new();
        
        let metadata = ReferenceNftMetadata {
            app_id: "n/abc123/tx:0".to_string(),
            name: Some("Test Token".to_string()),
            symbol: Some("TEST".to_string()),
            description: Some("A test token".to_string()),
            decimals: 8,
        };
        
        cache.cache_nft("n/abc123/tx:0", metadata);
        
        let retrieved = cache.get_for_token("t/abc123/tx:0");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, Some("Test Token".to_string()));
    }

    #[test]
    fn test_mark_as_reference() {
        let cache = ReferenceNftCache::new();
        
        assert!(cache.should_mark_as_reference("t/abc123/tx:0"));
        cache.mark_as_reference("abc123");
        assert!(!cache.should_mark_as_reference("t/abc123/tx:0"));
    }
}
