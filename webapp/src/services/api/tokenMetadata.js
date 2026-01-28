'use client';

import { ENDPOINTS } from '../apiConfig';

// Cache for NFT reference metadata to avoid repeated API calls
const nftReferenceCache = new Map();

/**
 * Convert a token app_id (t/...) to its NFT reference app_id (n/...)
 * @param {string} tokenAppId - Token app_id starting with t/
 * @returns {string|null} - NFT reference app_id starting with n/, or null if invalid
 */
export function getRefNftAppId(tokenAppId) {
    if (!tokenAppId || typeof tokenAppId !== 'string') return null;
    
    if (tokenAppId.startsWith('t/')) {
        return 'n/' + tokenAppId.slice(2);
    }
    
    return null;
}

/**
 * Fetch NFT reference metadata for a token
 * @param {string} tokenAppId - Token app_id (t/...)
 * @returns {Promise<Object|null>} - NFT metadata with name, symbol, description, image_url
 */
export async function fetchNftReferenceMetadata(tokenAppId) {
    const nftAppId = getRefNftAppId(tokenAppId);
    if (!nftAppId) return null;
    
    // Check cache first
    if (nftReferenceCache.has(nftAppId)) {
        return nftReferenceCache.get(nftAppId);
    }
    
    try {
        // Fetch from assets endpoint
        const response = await fetch(`${ENDPOINTS.ASSETS}?limit=500`);
        if (!response.ok) return null;
        
        const data = await response.json();
        const assets = data?.data?.assets || [];
        
        // Find the NFT reference
        const nftRef = assets.find(asset => asset.app_id === nftAppId);
        
        if (nftRef) {
            const metadata = {
                name: nftRef.name,
                symbol: nftRef.symbol,
                description: nftRef.description,
                image_url: nftRef.image_url,
                total_supply: nftRef.total_supply,
                decimals: nftRef.decimals,
                network: nftRef.network,
                app_id: nftRef.app_id
            };
            
            // Cache the result
            nftReferenceCache.set(nftAppId, metadata);
            return metadata;
        }
        
        return null;
    } catch (error) {
        console.error('[TokenMetadata] Error fetching NFT reference:', error);
        return null;
    }
}

/**
 * Preload all NFT reference metadata into cache
 * Call this once on app load to avoid multiple API calls
 */
export async function preloadNftReferenceCache() {
    try {
        const response = await fetch(`${ENDPOINTS.ASSETS}?limit=500`);
        if (!response.ok) return;
        
        const data = await response.json();
        const assets = data?.data?.assets || [];
        
        // Cache all NFTs (n/ prefix)
        assets.forEach(asset => {
            if (asset.app_id?.startsWith('n/')) {
                nftReferenceCache.set(asset.app_id, {
                    name: asset.name,
                    symbol: asset.symbol,
                    description: asset.description,
                    image_url: asset.image_url,
                    total_supply: asset.total_supply,
                    decimals: asset.decimals,
                    network: asset.network,
                    app_id: asset.app_id
                });
            }
        });
        
        console.log(`[TokenMetadata] Preloaded ${nftReferenceCache.size} NFT references`);
    } catch (error) {
        console.error('[TokenMetadata] Error preloading cache:', error);
    }
}

/**
 * Get cached NFT reference metadata (synchronous, for use after preload)
 * @param {string} tokenAppId - Token app_id (t/...)
 * @returns {Object|null} - Cached NFT metadata or null
 */
export function getCachedNftReference(tokenAppId) {
    const nftAppId = getRefNftAppId(tokenAppId);
    if (!nftAppId) return null;
    return nftReferenceCache.get(nftAppId) || null;
}

/**
 * Clear the NFT reference cache
 */
export function clearNftReferenceCache() {
    nftReferenceCache.clear();
}
