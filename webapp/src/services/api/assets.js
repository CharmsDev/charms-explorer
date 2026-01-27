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
