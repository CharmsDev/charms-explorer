'use client';

import { ENDPOINTS } from './apiConfig';
import { fixJsonResponse, handleApiError, paginateItems } from './apiUtils';
import { transformCharmsArray, countCharmsByType, createDefaultCharm, detectCharmType } from './transformers';

// Core API service functions

// Fetches raw charm data from the API
export const fetchRawCharmsData = async () => {
    try {
        const response = await fetch(ENDPOINTS.CHARMS);

        // Even if response is not OK, try to parse it for error details
        const responseText = await response.text();

        try {
            const data = JSON.parse(responseText);

            // Check if the response contains an error message
            if (!response.ok) {
                console.warn(`API error (${response.status}): ${data.error || 'Unknown error'}`);
                // Return empty data structure instead of throwing
                return { charms: [] };
            }

            return data;
        } catch (parseError) {
            console.error('JSON parse error:', parseError);

            // If direct parsing fails, try to fix the JSON
            try {
                const fixedJson = fixJsonResponse(responseText);

                // Parse the fixed JSON
                const data = JSON.parse(fixedJson);

                // Check if the response contains an error message
                if (!response.ok) {
                    console.warn(`API error (${response.status}): ${data.error || 'Unknown error'}`);
                    // Return empty data structure instead of throwing
                    return { charms: [] };
                }

                return data;
            } catch (error) {
                console.error('Error fixing JSON:', error);
                // Return empty data structure instead of throwing
                return { charms: [] };
            }
        }
    } catch (error) {
        console.error('Error fetching charms data:', error);
        // Return empty data structure instead of throwing
        return { charms: [] };
    }
};

// Fetches assets from the new assets endpoint with type filtering
export const fetchAssetsByType = async (assetType, page = 1, limit = 20, sort = 'newest', network = null) => {
    try {

        // Build URL with parameters
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

// Fetches and transforms charm assets with pagination and sorting (for "All" tab)
export const fetchAssets = async (page = 1, limit = 20, sort = 'newest', network = null) => {
    try {

        // Build URL with network parameter if provided
        let url = `${ENDPOINTS.CHARMS}`;
        const params = new URLSearchParams();

        if (network) {
            params.append('network', network);
        }

        if (params.toString()) {
            url += `?${params.toString()}`;
        }

        const response = await fetch(url);

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const data = await response.json();

        if (!data.data || !data.data.charms) {
            console.warn('[API] No charms data in response');
            return {
                assets: [],
                total: 0,
                page: 1,
                totalPages: 1
            };
        }

        let charms = data.data.charms;

        const transformedCharms = transformCharmsArray(charms);

        // Apply client-side sorting only if needed (API likely handles this too but keeps consistent format)
        const sortedCharms = transformedCharms.sort((a, b) => {
            if (sort === 'oldest') {
                return a.block_height - b.block_height;
            } else {
                return b.block_height - a.block_height; // newest first (default)
            }
        });

        // NO client-side pagination needed as API already returns paginated results
        // The API returns the specific page requested, so we use the full result set
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
        console.error('[API] Error fetching assets:', error);
        throw error;
    }
};

// Gets a specific asset by ID
export const getAssetById = async (id) => {
    try {
        // Extract the charm ID from the URL format if needed
        const charmId = id.startsWith('charm-') ? id : id;

        // Use the dedicated endpoint to fetch the charm by its ID
        const response = await fetch(ENDPOINTS.CHARM_BY_CHARMID(charmId));

        if (!response.ok) {
            console.warn(`Charm not found with ID: ${charmId}, using default charm`);
            return createDefaultCharm(id);
        }

        const charm = await response.json();

        // Transform the charm data
        return transformCharmsArray([charm])[0];
    } catch (error) {
        console.error('Error fetching asset by ID:', error);
        // Fallback to the old method if the new endpoint fails
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

// Gets counts of assets by type from the new assets endpoint
export const getAssetCounts = async () => {
    try {
        const response = await fetch(ENDPOINTS.ASSET_COUNTS);

        if (!response.ok) {
            console.warn('Asset counts endpoint not available, falling back to charms data');
            // Fallback to old method
            const data = await fetchRawCharmsData();
            return countCharmsByType(data.charms || []);
        }

        const counts = await response.json();
        return counts;
    } catch (error) {
        console.error('Error getting asset counts:', error);
        // Fallback to old method
        try {
            const data = await fetchRawCharmsData();
            return countCharmsByType(data.charms || []);
        } catch (fallbackError) {
            console.error('Fallback asset counts also failed:', fallbackError);
            return { total: 0, nft: 0, token: 0, dapp: 0 };
        }
    }
};

// Fetches indexer status information
export const fetchIndexerStatus = async () => {
    try {
        const response = await fetch(ENDPOINTS.STATUS);

        if (!response.ok) {
            throw new Error(`API error: ${response.status}`);
        }

        const data = await response.json();
        return data;
    } catch (error) {
        throw handleApiError(error, 'fetch indexer status');
    }
};

// Resets the indexer (clears bookmark table)
export const resetIndexer = async () => {
    try {
        const response = await fetch(ENDPOINTS.RESET, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            }
        });

        if (!response.ok) {
            throw new Error(`API error: ${response.status}`);
        }

        const data = await response.json();
        return data;
    } catch (error) {
        throw handleApiError(error, 'reset indexer');
    }
};

// Likes a charm
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
        console.error('Error liking charm:', error);
        throw handleApiError(error, 'like charm');
    }
};

// Unlikes a charm
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
        console.error('Error unliking charm:', error);
        throw handleApiError(error, 'unlike charm');
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
        console.error('[API] Error fetching charms by address:', error);
        throw error;
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
