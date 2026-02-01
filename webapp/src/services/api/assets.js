'use client';

import { ENDPOINTS } from '../apiConfig';
import { handleApiError } from '../apiUtils';
import { transformCharmsArray, countCharmsByType, createDefaultCharm } from '../transformers';
import { fetchRawCharmsData } from './charms';

export const fetchAssetsByType = async (assetType, page = 1, limit = 20, sort = 'newest', network = null) => {
    try {
        let url = `${ENDPOINTS.ASSETS}`;
        const params = new URLSearchParams();

        if (assetType && assetType !== 'all') {
            params.append('asset_type', assetType);
        }

        if (network && network !== 'all') {
            params.append('network', network);
        }

        params.append('page', page.toString());
        params.append('limit', limit.toString());
        params.append('sort', sort);

        if (params.toString()) {
            url += `?${params.toString()}`;
        }

        const response = await fetch(url);

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const data = await response.json();

        if (!data.data || !data.data.assets) {
            console.warn('[API] No assets data in response');
            return {
                assets: [],
                total: 0,
                page: 1,
                totalPages: 1
            };
        }

        const assets = data.data.assets;
        
        // Map assets to expected format
        // Images for tokens will be fetched on-demand from reference NFT by AssetCard
        const mappedAssets = assets.map(asset => ({
            ...asset,
            id: asset.app_id || asset.id,
            type: asset.asset_type,
            image: asset.image_url, // May be null for tokens, will be fetched from reference NFT
            ticker: asset.symbol,
            createdAt: asset.created_at,
            supply: asset.total_supply,
            app_id: asset.app_id
        }));

        return {
            assets: mappedAssets,
            total: data.pagination.total,
            page: data.pagination.page,
            totalPages: data.pagination.total_pages
        };
    } catch (error) {
        console.error('[API] Error fetching assets by type:', error);
        throw error;
    }
};

export const getAssetById = async (id) => {
    try {
        const charmId = id.startsWith('charm-') ? id : id;

        const response = await fetch(ENDPOINTS.CHARM_BY_CHARMID(charmId));

        if (!response.ok) {
            console.warn(`Charm not found with ID: ${charmId}, using default charm`);
            return createDefaultCharm(id);
        }

        const charm = await response.json();
        return transformCharmsArray([charm])[0];
    } catch (error) {
        console.error('Error fetching asset by ID:', error);
        try {
            const data = await fetchRawCharmsData();
            const charm = data.charms.find(charm => charm.charmid === id);

            if (!charm) {
                return createDefaultCharm(id);
            }

            return transformCharmsArray([charm])[0];
        } catch (fallbackError) {
            throw handleApiError(fallbackError, 'fetch asset details');
        }
    }
};

export const getAssetCounts = async () => {
    try {
        const response = await fetch(ENDPOINTS.ASSET_COUNTS);

        if (!response.ok) {
            console.warn('Asset counts endpoint not available, falling back to charms data');
            const data = await fetchRawCharmsData();
            return countCharmsByType(data.charms || []);
        }

        const counts = await response.json();
        return counts;
    } catch (error) {
        console.error('Error getting asset counts:', error);
        try {
            const data = await fetchRawCharmsData();
            return countCharmsByType(data.charms || []);
        } catch (fallbackError) {
            console.error('Fallback asset counts also failed:', fallbackError);
            return { total: 0, nft: 0, token: 0, dapp: 0 };
        }
    }
};

export const fetchAssetHolders = async (appId) => {
    try {
        // [RJJ-STATS-HOLDERS] Normalize app_id to use :0 suffix for holders lookup
        // Holders are consolidated under the base token app_id (ending in :0)
        let normalizedAppId = appId;
        if (appId && appId.match(/:[0-9]+$/)) {
            normalizedAppId = appId.replace(/:[0-9]+$/, ':0');
        }
        const response = await fetch(`${ENDPOINTS.ASSET_HOLDERS(normalizedAppId)}`);

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const data = await response.json();
        return data;
    } catch (error) {
        console.error('[API] Error fetching asset holders:', error);
        throw error;
    }
};

/**
 * Fetch asset data by app_id from the /assets endpoint
 * This returns asset data including total_supply, decimals, etc.
 */
export const fetchAssetByAppId = async (appId) => {
    try {
        const encodedAppId = encodeURIComponent(appId);
        const response = await fetch(`${ENDPOINTS.ASSETS}?app_id=${encodedAppId}&limit=1`);

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const data = await response.json();
        
        // Return the first matching asset
        if (data?.data?.assets && data.data.assets.length > 0) {
            return data.data.assets[0];
        }
        
        return null;
    } catch (error) {
        console.error('[API] Error fetching asset by app_id:', error);
        return null;
    }
};

/**
 * Extract the unique identifier from an app_id
 * For tokens (t/HASH/TXID:VOUT), returns t/HASH (grouping all mints of same token)
 * For NFTs (n/HASH/TXID:VOUT), returns n/HASH
 * For dApps and others, returns the full app_id
 */
const getUniqueAppIdKey = (appId) => {
    if (!appId) return null;
    
    // For t/ and n/ prefixed IDs, extract the HASH part (before the second /)
    if (appId.startsWith('t/') || appId.startsWith('n/')) {
        const parts = appId.split('/');
        if (parts.length >= 2) {
            // Return prefix + first hash (t/HASH or n/HASH)
            return `${parts[0]}/${parts[1]}`;
        }
    }
    
    return appId;
};

/**
 * Extract base hash from app_id (without prefix and without txid:vout)
 */
const getBaseHash = (appId) => {
    if (!appId) return null;
    const parts = appId.split('/');
    if (parts.length >= 2) {
        return parts[1]; // Return just the HASH part
    }
    return null;
};

/**
 * Fetch unique assets (grouping tokens/NFTs by their reference)
 * This filters out duplicate mints of the same token type
 * Also filters out NFT references that are just metadata for tokens
 */
export const fetchUniqueAssets = async (assetType = 'all', page = 1, limit = 20, sort = 'newest', network = null) => {
    try {
        // Fetch ALL assets to properly deduplicate and filter
        const fetchLimit = 500;
        
        let url = `${ENDPOINTS.ASSETS}?limit=${fetchLimit}`;
        if (network && network !== 'all') {
            url += `&network=${network}`;
        }

        const response = await fetch(url);

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const data = await response.json();

        if (!data.data || !data.data.assets) {
            return {
                assets: [],
                total: 0,
                page: 1,
                totalPages: 1
            };
        }

        const allAssets = data.data.assets;
        
        // Build a set of hashes that have tokens (t/HASH)
        // These NFTs (n/HASH) are just references, not real NFTs
        const tokenHashes = new Set();
        for (const asset of allAssets) {
            if (asset.app_id?.startsWith('t/')) {
                const hash = getBaseHash(asset.app_id);
                if (hash) tokenHashes.add(hash);
            }
        }
        
        // Deduplicate and filter based on type
        const seen = new Map();
        const uniqueAssets = [];
        
        for (const asset of allAssets) {
            const appId = asset.app_id;
            if (!appId) continue;
            
            const key = getUniqueAppIdKey(appId);
            if (!key || seen.has(key)) continue;
            
            // Determine actual type based on app_id prefix
            let actualType = 'other';
            if (appId.startsWith('n/')) {
                // Check if this NFT is just a token reference
                const hash = getBaseHash(appId);
                if (tokenHashes.has(hash)) {
                    // Skip - this is a token reference NFT, not a real NFT
                    continue;
                }
                actualType = 'nft';
            } else if (appId.startsWith('t/')) {
                actualType = 'token';
            } else if (appId.startsWith('b/')) {
                actualType = 'dapp';
            }
            
            // Filter by requested type
            if (assetType !== 'all' && actualType !== assetType) {
                continue;
            }
            
            seen.set(key, true);
            // Ensure asset has correct type for display
            uniqueAssets.push({ ...asset, asset_type: actualType });
        }
        
        // Sort
        if (sort === 'newest') {
            uniqueAssets.sort((a, b) => (b.block_height || 0) - (a.block_height || 0));
        } else {
            uniqueAssets.sort((a, b) => (a.block_height || 0) - (b.block_height || 0));
        }
        
        // Apply pagination
        const startIndex = (page - 1) * limit;
        const paginatedAssets = uniqueAssets.slice(startIndex, startIndex + limit);
        const totalUnique = uniqueAssets.length;
        
        return {
            assets: paginatedAssets,
            total: totalUnique,
            page: page,
            totalPages: Math.ceil(totalUnique / limit)
        };
    } catch (error) {
        console.error('[API] Error fetching unique assets:', error);
        throw error;
    }
};

/**
 * Get counts of unique assets (not individual charms)
 * Excludes NFT references that are just metadata for tokens
 */
export const getUniqueAssetCounts = async (network = null) => {
    try {
        const fetchLimit = 500;
        
        let url = `${ENDPOINTS.ASSETS}?limit=${fetchLimit}`;
        if (network && network !== 'all') {
            url += `&network=${network}`;
        }

        const response = await fetch(url);

        if (!response.ok) {
            return { total: 0, nft: 0, token: 0, dapp: 0 };
        }

        const data = await response.json();
        const allAssets = data.data?.assets || [];
        
        // Build set of hashes that have tokens
        const tokenHashes = new Set();
        for (const asset of allAssets) {
            if (asset.app_id?.startsWith('t/')) {
                const hash = getBaseHash(asset.app_id);
                if (hash) tokenHashes.add(hash);
            }
        }
        
        // Count unique by actual type (based on app_id prefix)
        const seenByType = {
            nft: new Set(),
            token: new Set(),
            dapp: new Set()
        };
        
        for (const asset of allAssets) {
            const appId = asset.app_id;
            if (!appId) continue;
            
            const key = getUniqueAppIdKey(appId);
            if (!key) continue;
            
            // Determine actual type based on prefix
            if (appId.startsWith('n/')) {
                const hash = getBaseHash(appId);
                // Skip NFT references that are just token metadata
                if (!tokenHashes.has(hash)) {
                    seenByType.nft.add(key);
                }
            } else if (appId.startsWith('t/')) {
                seenByType.token.add(key);
            } else if (appId.startsWith('b/')) {
                seenByType.dapp.add(key);
            }
        }
        
        return {
            total: seenByType.nft.size + seenByType.token.size + seenByType.dapp.size,
            nft: seenByType.nft.size,
            token: seenByType.token.size,
            dapp: seenByType.dapp.size
        };
    } catch (error) {
        console.error('[API] Error getting unique asset counts:', error);
        return { total: 0, nft: 0, token: 0, dapp: 0 };
    }
};
