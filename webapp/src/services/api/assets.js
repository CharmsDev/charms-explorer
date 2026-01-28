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

        return {
            assets: assets,
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
        const response = await fetch(`${ENDPOINTS.ASSET_HOLDERS(appId)}`);

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
 * Fetch unique assets (grouping tokens/NFTs by their reference)
 * This filters out duplicate mints of the same token type
 */
export const fetchUniqueAssets = async (assetType = 'all', page = 1, limit = 20, sort = 'newest', network = null) => {
    try {
        // Fetch more items than needed to account for duplicates
        const fetchLimit = Math.min(limit * 5, 500);
        
        let url = `${ENDPOINTS.ASSETS}`;
        const params = new URLSearchParams();

        if (assetType && assetType !== 'all') {
            params.append('asset_type', assetType);
        }

        if (network && network !== 'all') {
            params.append('network', network);
        }

        params.append('page', '1'); // Always fetch from first page for deduplication
        params.append('limit', fetchLimit.toString());
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
            return {
                assets: [],
                total: 0,
                page: 1,
                totalPages: 1
            };
        }

        const allAssets = data.data.assets;
        
        // Deduplicate: keep only unique assets by their reference key
        const seen = new Map();
        const uniqueAssets = [];
        
        for (const asset of allAssets) {
            const key = getUniqueAppIdKey(asset.app_id);
            if (key && !seen.has(key)) {
                seen.set(key, true);
                uniqueAssets.push(asset);
            }
        }
        
        // Apply pagination to unique results
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
 */
export const getUniqueAssetCounts = async (network = null) => {
    try {
        // Fetch all assets to count unique ones
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
        
        // Count unique by type
        const seenByType = {
            nft: new Set(),
            token: new Set(),
            dapp: new Set()
        };
        
        for (const asset of allAssets) {
            const key = getUniqueAppIdKey(asset.app_id);
            const type = asset.asset_type || 'other';
            
            if (key && seenByType[type]) {
                seenByType[type].add(key);
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
