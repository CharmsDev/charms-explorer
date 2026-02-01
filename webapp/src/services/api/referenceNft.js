'use client';

import { ENDPOINTS } from '../apiConfig';

/**
 * Reference NFT Service
 * 
 * Fetches and caches reference NFT metadata for tokens.
 * When displaying a token, the image should be fetched from its reference NFT
 * to avoid storing duplicate images in the database.
 */

// In-memory cache for reference NFT metadata
// Key: hash (extracted from app_id), Value: { image_url, name, symbol, ... }
const referenceNftCache = new Map();

// Set of hashes currently being fetched (to avoid duplicate requests)
const pendingFetches = new Set();

/**
 * Extract hash from app_id
 * Works for both n/ and t/ prefixes: n/HASH/txid:vout -> HASH
 * @param {string} appId 
 * @returns {string|null}
 */
export function extractHashFromAppId(appId) {
    if (!appId || appId.length < 3) return null;
    
    // Skip prefix (n/ or t/)
    const withoutPrefix = appId.substring(2);
    
    // Hash is the part before the first /
    const slashPos = withoutPrefix.indexOf('/');
    if (slashPos > 0) {
        return withoutPrefix.substring(0, slashPos);
    }
    
    // No second slash, entire remainder is hash
    return withoutPrefix;
}

/**
 * Get cached reference NFT metadata
 * @param {string} hash 
 * @returns {Object|null}
 */
export function getCachedReferenceNft(hash) {
    return referenceNftCache.get(hash) || null;
}

/**
 * Fetch reference NFT metadata by hash
 * Uses cache to avoid duplicate requests
 * @param {string} hash 
 * @returns {Promise<Object|null>}
 */
export async function fetchReferenceNftByHash(hash) {
    if (!hash) return null;
    
    // Check cache first
    if (referenceNftCache.has(hash)) {
        return referenceNftCache.get(hash);
    }
    
    // Check if already fetching
    if (pendingFetches.has(hash)) {
        // Wait for the pending fetch to complete
        await new Promise(resolve => setTimeout(resolve, 100));
        return referenceNftCache.get(hash) || null;
    }
    
    try {
        pendingFetches.add(hash);
        
        const response = await fetch(ENDPOINTS.REFERENCE_NFT(hash));
        
        if (!response.ok) {
            console.warn(`[ReferenceNFT] Not found for hash: ${hash.substring(0, 16)}...`);
            return null;
        }
        
        const data = await response.json();
        
        // Cache the result
        referenceNftCache.set(hash, data);
        
        return data;
    } catch (error) {
        console.error(`[ReferenceNFT] Error fetching for hash ${hash.substring(0, 16)}...:`, error);
        return null;
    } finally {
        pendingFetches.delete(hash);
    }
}

/**
 * Get reference NFT image for a token
 * Extracts hash from token app_id and fetches the reference NFT image
 * @param {string} tokenAppId - Token app_id (t/HASH/...)
 * @returns {Promise<string|null>} - Image URL or null
 */
export async function getReferenceNftImage(tokenAppId) {
    if (!tokenAppId?.startsWith('t/')) return null;
    
    const hash = extractHashFromAppId(tokenAppId);
    if (!hash) return null;
    
    const refNft = await fetchReferenceNftByHash(hash);
    return refNft?.image_url || null;
}

/**
 * Get reference NFT metadata for a token (cached)
 * @param {string} tokenAppId - Token app_id (t/HASH/...)
 * @returns {Object|null}
 */
export function getCachedReferenceNftForToken(tokenAppId) {
    if (!tokenAppId?.startsWith('t/')) return null;
    
    const hash = extractHashFromAppId(tokenAppId);
    if (!hash) return null;
    
    return getCachedReferenceNft(hash);
}

/**
 * Prefetch reference NFT metadata for multiple tokens
 * Useful for batch loading when displaying a list of tokens
 * @param {string[]} tokenAppIds 
 */
export async function prefetchReferenceNfts(tokenAppIds) {
    const uniqueHashes = new Set();
    
    for (const appId of tokenAppIds) {
        if (appId?.startsWith('t/')) {
            const hash = extractHashFromAppId(appId);
            if (hash && !referenceNftCache.has(hash)) {
                uniqueHashes.add(hash);
            }
        }
    }
    
    // Fetch all unique hashes in parallel
    await Promise.all(
        Array.from(uniqueHashes).map(hash => fetchReferenceNftByHash(hash))
    );
}

/**
 * Clear the reference NFT cache
 * Useful for testing or when data needs to be refreshed
 */
export function clearReferenceNftCache() {
    referenceNftCache.clear();
}

/**
 * Get cache statistics
 * @returns {{ size: number }}
 */
export function getReferenceNftCacheStats() {
    return {
        size: referenceNftCache.size
    };
}
