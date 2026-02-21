'use client';

import { ENDPOINTS } from '../apiConfig';
import { handleApiError, logger } from '../apiUtils';
import { transformCharmsArray } from '../transformers';

export const fetchAssets = async (page = 1, limit = 20, sort = 'newest', network = null) => {
    try {
        let url = `${ENDPOINTS.CHARMS}`;
        const params = new URLSearchParams();

        params.append('page', page.toString());
        params.append('limit', limit.toString());
        params.append('sort', sort);

        if (network) {
            params.append('network', network);
        }

        url += `?${params.toString()}`;

        const response = await fetch(url);

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const data = await response.json();

        if (!data.data || !data.data.charms) {
            logger.warn('fetchAssets', 'No charms data in response');
            return {
                assets: [],
                total: 0,
                page: 1,
                totalPages: 1
            };
        }

        let charms = data.data.charms;
        const transformedCharms = transformCharmsArray(charms);

        const sortedCharms = transformedCharms.sort((a, b) => {
            // Treat null block_height (mempool) as highest value so it sorts first (newest)
            const aH = a.block_height ?? Infinity;
            const bH = b.block_height ?? Infinity;
            if (sort === 'oldest') {
                return aH - bH;
            } else {
                return bH - aH;
            }
        });

        const paginatedCharms = sortedCharms;
        const totalPages = data.pagination?.total_pages || Math.ceil(data.pagination?.total / limit) || 1;
        const totalCount = data.pagination?.total || sortedCharms.length;

        return {
            assets: paginatedCharms,
            total: totalCount,
            page: page,
            totalPages: totalPages
        };
    } catch (error) {
        logger.error('fetchAssets', error);
        throw error;
    }
};

export const getCharmsCountByType = async (network = null) => {
    try {
        const url = network 
            ? `${ENDPOINTS.CHARMS_COUNT_BY_TYPE}?network=${network}`
            : ENDPOINTS.CHARMS_COUNT_BY_TYPE;
        
        const response = await fetch(url);

        if (!response.ok) {
            logger.warn('getCharmsCountByType', 'Endpoint not available');
            return { total: 0, nft: 0, token: 0, dapp: 0 };
        }

        const counts = await response.json();
        return counts;
    } catch (error) {
        logger.error('getCharmsCountByType', error);
        return { total: 0, nft: 0, token: 0, dapp: 0 };
    }
};

export const fetchCharmsByAddress = async (address) => {
    try {
        const response = await fetch(`${ENDPOINTS.CHARMS_BY_ADDRESS(address)}`);

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const data = await response.json();
        return data;
    } catch (error) {
        logger.error('fetchCharmsByAddress', error);
        throw error;
    }
};

export const getCharmByTxId = async (txid) => {
    try {
        const response = await fetch(`${ENDPOINTS.CHARM_BY_TXID(txid)}`);

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const data = await response.json();
        return data;
    } catch (error) {
        logger.error('getCharmByTxId', error);
        throw error;
    }
};

export const likeCharm = async (charmId, userId = 1) => {
    try {
        const response = await fetch(ENDPOINTS.LIKE_CHARM, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                charm_id: charmId,
                user_id: userId
            })
        });

        if (!response.ok) {
            throw new Error(`API error: ${response.status}`);
        }

        const data = await response.json();
        return data;
    } catch (error) {
        logger.error('likeCharm', error);
        throw handleApiError(error, 'like charm');
    }
};

export const unlikeCharm = async (charmId, userId = 1) => {
    try {
        const response = await fetch(ENDPOINTS.LIKE_CHARM, {
            method: 'DELETE',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                charm_id: charmId,
                user_id: userId
            })
        });

        if (!response.ok) {
            throw new Error(`API error: ${response.status}`);
        }

        const data = await response.json();
        return data;
    } catch (error) {
        logger.error('unlikeCharm', error);
        throw handleApiError(error, 'unlike charm');
    }
};
