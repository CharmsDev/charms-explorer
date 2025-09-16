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
export const fetchAssets = async (page = 1, limit = 20, sort = 'newest', network = null) => {
  try {
    console.log(`[API] Fetching assets - page: ${page}, limit: ${limit}, sort: ${sort}, network: ${network}`);
    
    // Build URL with network parameter if provided
    let url = `${ENDPOINTS.CHARMS}`;
    const params = new URLSearchParams();
    
    if (network) {
      params.append('network', network);
    }
    
    if (params.toString()) {
      url += `?${params.toString()}`;
    }
    
    console.log(`[API] Request URL: ${url}`);
    
    const response = await fetch(url);
    
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    
    const data = await response.json();
    console.log('[API] Raw response data:', data);
    
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
    console.log(`[API] Received ${charms.length} charms from API`);
    
    // Transform the charms data
    const transformedCharms = transformCharmsArray(charms);
    console.log(`[API] Transformed ${transformedCharms.length} charms`);
    
    // Apply client-side sorting
    const sortedCharms = transformedCharms.sort((a, b) => {
      if (sort === 'oldest') {
        return a.block_height - b.block_height;
      } else {
        return b.block_height - a.block_height; // newest first (default)
      }
    });
    
    // Apply client-side pagination
    const startIndex = (page - 1) * limit;
    const endIndex = startIndex + limit;
    const paginatedCharms = sortedCharms.slice(startIndex, endIndex);
    
    const totalPages = Math.ceil(sortedCharms.length / limit);
    
    // Use API total count if available, otherwise use client-side count
    const totalCount = data.pagination?.total || sortedCharms.length;
    
    console.log(`[API] Returning page ${page} with ${paginatedCharms.length} charms, total: ${totalCount}, totalPages: ${totalPages}`);
    
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
