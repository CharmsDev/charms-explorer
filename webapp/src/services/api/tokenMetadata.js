'use client';

import { ENDPOINTS } from '../apiConfig';
import { logger } from '../apiUtils';

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
 * Extract metadata from spell native_data
 * The image and other metadata are stored in the spell outputs
 */
function extractMetadataFromSpell(charmData) {
    // Structure is charm.data.native_data
    const nativeData = charmData?.data?.native_data;
    
    if (!nativeData?.tx?.outs) return null;
    
    const outs = nativeData.tx.outs;
    
    // Look for metadata in outputs (usually in outs[0]["0"])
    for (const out of outs) {
        const data = out["0"] || out[0];
        if (data && typeof data === 'object') {
            return {
                name: data.name,
                symbol: data.ticker || data.symbol,
                description: data.description,
                image_url: data.image, // base64 image data
                total_supply: data.supply_limit,
                decimals: data.decimals,
                url: data.url
            };
        }
    }
    
    return null;
}

/**
 * Fetch charm data by charmid (app_id) to get spell data with metadata
 */
async function fetchCharmByAppId(appId) {
    try {
        // The charmid format in API is the app_id
        const response = await fetch(ENDPOINTS.CHARM_BY_CHARMID(appId));
        if (!response.ok) return null;
        
        const charm = await response.json();
        return charm;
    } catch (error) {
        logger.error('TokenMetadata.fetchCharmByAppId', error);
        return null;
    }
}

/**
 * Fetch NFT reference metadata for a token
 * First tries the assets endpoint, then fetches the charm directly for spell data
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
        // First try: fetch from assets endpoint
        const assetsResponse = await fetch(`${ENDPOINTS.ASSETS}?limit=500`);
        if (assetsResponse.ok) {
            const data = await assetsResponse.json();
            const assets = data?.data?.assets || [];
            const nftRef = assets.find(asset => asset.app_id === nftAppId);
            
            if (nftRef && nftRef.image_url) {
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
                nftReferenceCache.set(nftAppId, metadata);
                return metadata;
            }
        }
        
        // Second try: fetch the NFT charm directly to get spell data
        const charm = await fetchCharmByAppId(nftAppId);
        if (charm) {
            // Extract metadata from spell data
            const spellMetadata = extractMetadataFromSpell(charm.spell_data || charm);
            
            if (spellMetadata) {
                const metadata = {
                    ...spellMetadata,
                    network: charm.network,
                    app_id: nftAppId
                };
                nftReferenceCache.set(nftAppId, metadata);
                return metadata;
            }
            
            // Fallback to charm's direct properties
            if (charm.name || charm.image_url) {
                const metadata = {
                    name: charm.name,
                    symbol: charm.symbol || charm.ticker,
                    description: charm.description,
                    image_url: charm.image_url || charm.image,
                    total_supply: charm.total_supply,
                    decimals: charm.decimals,
                    network: charm.network,
                    app_id: nftAppId
                };
                nftReferenceCache.set(nftAppId, metadata);
                return metadata;
            }
        }
        
        return null;
    } catch (error) {
        logger.error('TokenMetadata.fetchNftReference', error);
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
        
        logger.info('TokenMetadata', `Preloaded ${nftReferenceCache.size} NFT references`);
    } catch (error) {
        logger.error('TokenMetadata.preloadCache', error);
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

// Cache for spell images to avoid repeated API calls
const spellImageCache = new Map();

/**
 * Fetch the image from a charm's spell data
 * Used for NFTs that don't have image_url in the assets endpoint
 * @param {string} appId - The app_id of the charm (n/...)
 * @returns {Promise<string|null>} - Base64 image data or null
 */
export async function fetchCharmSpellImage(appId) {
    if (!appId) return null;
    
    // Check cache first
    if (spellImageCache.has(appId)) {
        return spellImageCache.get(appId);
    }
    
    try {
        const response = await fetch(ENDPOINTS.CHARM_BY_CHARMID(appId));
        if (!response.ok) return null;
        
        const charm = await response.json();
        
        // Extract image from spell data: charm.data.native_data.tx.outs[0]["0"].image
        const nativeData = charm?.data?.native_data;
        if (nativeData?.tx?.outs) {
            for (const out of nativeData.tx.outs) {
                const data = out["0"] || out[0];
                if (data?.image) {
                    spellImageCache.set(appId, data.image);
                    return data.image;
                }
            }
        }
        
        return null;
    } catch (error) {
        logger.error('TokenMetadata.fetchCharmSpellImage', error);
        return null;
    }
}
