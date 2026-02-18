'use client';

import { ENDPOINTS } from '../apiConfig';
import { logger } from '../apiUtils';
import { transformCharmsArray, createDefaultCharm } from '../transformers';

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
            logger.warn('fetchAssetsByType', 'No assets data in response');
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
        logger.error('fetchAssetsByType', error);
        throw error;
    }
};

export const getAssetById = async (id) => {
    try {
        const charmId = id.startsWith('charm-') ? id : id;

        const response = await fetch(ENDPOINTS.CHARM_BY_CHARMID(charmId));

        if (!response.ok) {
            logger.warn('getAssetById', `Charm not found: ${charmId}`);
            return createDefaultCharm(id);
        }

        const charm = await response.json();
        return transformCharmsArray([charm])[0];
    } catch (error) {
        logger.error('getAssetById', error);
        return createDefaultCharm(id);
    }
};

export const getAssetCounts = async (network = null) => {
    try {
        let url = ENDPOINTS.ASSET_COUNTS;
        if (network && network !== 'all') {
            url += `?network=${encodeURIComponent(network)}`;
        }
        const response = await fetch(url);

        if (!response.ok) {
            logger.warn('getAssetCounts', 'Endpoint not available');
            return { total: 0, nft: 0, token: 0, dapp: 0 };
        }

        const counts = await response.json();
        return counts;
    } catch (error) {
        logger.error('getAssetCounts', error);
        return { total: 0, nft: 0, token: 0, dapp: 0 };
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
        logger.error('fetchAssetHolders', error);
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
        logger.error('fetchAssetByAppId', error);
        return null;
    }
};

