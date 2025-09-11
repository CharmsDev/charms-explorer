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
        console.log('Raw API response:', responseText);

        try {
            // Try to parse the JSON directly first
            const data = JSON.parse(responseText);
            console.log('Parsed JSON data:', data);

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
                console.log('Fixed JSON:', fixedJson);

                // Parse the fixed JSON
                const data = JSON.parse(fixedJson);
                console.log('Parsed fixed JSON data:', data);

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

// Fetches and transforms charm assets with pagination and sorting
export const fetchAssets = async (type = 'all', page = 1, limit = 12, sort = 'newest') => {
    try {
        let endpoint;
        if (type === 'all') {
            endpoint = ENDPOINTS.buildPaginatedUrl(ENDPOINTS.CHARMS, page, limit, sort);
        } else {
            // For type-specific endpoints, we need to handle the query params differently
            const baseUrl = ENDPOINTS.CHARMS_BY_TYPE(type).split('?')[0];
            const typeParam = `type=${encodeURIComponent(type)}`;
            const paginationParams = new URLSearchParams({
                page,
                limit,
                sort
            }).toString();
            endpoint = `${baseUrl}?${typeParam}&${paginationParams}`;
        }

        const response = await fetch(endpoint);

        if (!response.ok) {
            throw new Error(`API error: ${response.status}`);
        }

        const data = await response.json();
        console.log('Raw API response data:', data);

        // Transform the charms data
        const transformedCharms = transformCharmsArray(data.data.charms);

        // Ensure pagination metadata is properly structured
        const paginationData = data.pagination || {
            total: data.data.charms.length,
            page: page,
            limit: limit,
            total_pages: Math.ceil(data.data.charms.length / limit) || 1
        };

        console.log('Pagination data:', paginationData);
        console.log('Transformed charms count:', transformedCharms.length);

        // Return with pagination metadata
        return {
            data: transformedCharms,
            pagination: paginationData
        };
    } catch (error) {
        console.error('Error in fetchAssets:', error);

        // Fallback to client-side filtering if the API call fails
        try {
            const data = await fetchRawCharmsData();

            // Filter by type using the detection logic if needed
            let filteredCharms = data.charms;
            if (type !== 'all') {
                // Use detectCharmType for filtering
                filteredCharms = data.charms.filter(charm => detectCharmType(charm) === type);
            }

            // Sort the charms
            filteredCharms.sort((a, b) => {
                if (sort === 'oldest') {
                    return a.block_height - b.block_height;
                } else {
                    return b.block_height - a.block_height;
                }
            });

            // Transform the *filtered* data
            const transformedCharms = transformCharmsArray(filteredCharms);

            // Paginate the results
            const paginatedResult = paginateItems(transformedCharms, page, limit);
            console.log('Client-side pagination result:', paginatedResult);
            return paginatedResult;
        } catch (fallbackError) {
            throw handleApiError(fallbackError, 'fetch assets (fallback)');
        }
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

// Gets counts of assets by type
export const getAssetCounts = async () => {
    try {
        const data = await fetchRawCharmsData();
        return countCharmsByType(data.charms || []);
    } catch (error) {
        console.error('Error getting asset counts:', error);
        // Return default counts instead of throwing
        return { total: 0, nft: 0, token: 0, dapp: 0 };
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
