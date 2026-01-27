'use client';

import { ENDPOINTS } from '../apiConfig';
import { fixJsonResponse, handleApiError } from '../apiUtils';
import { transformCharmsArray, countCharmsByType, createDefaultCharm } from '../transformers';

export const fetchRawCharmsData = async () => {
    try {
        const response = await fetch(ENDPOINTS.CHARMS);
        const responseText = await response.text();

        try {
            const data = JSON.parse(responseText);

            if (!response.ok) {
                console.warn(`API error (${response.status}): ${data.error || 'Unknown error'}`);
                return { charms: [] };
            }

            return data;
        } catch (parseError) {
            console.error('JSON parse error:', parseError);

            try {
                const fixedJson = fixJsonResponse(responseText);
                const data = JSON.parse(fixedJson);

                if (!response.ok) {
                    console.warn(`API error (${response.status}): ${data.error || 'Unknown error'}`);
                    return { charms: [] };
                }

                return data;
            } catch (error) {
                console.error('Error fixing JSON:', error);
                return { charms: [] };
            }
        }
    } catch (error) {
        console.error('Error fetching charms data:', error);
        return { charms: [] };
    }
};

export const fetchAssets = async (page = 1, limit = 20, sort = 'newest', network = null) => {
    try {
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

        const sortedCharms = transformedCharms.sort((a, b) => {
            if (sort === 'oldest') {
                return a.block_height - b.block_height;
            } else {
                return b.block_height - a.block_height;
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
        console.error('[API] Error fetching assets:', error);
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
            console.warn('Charms count-by-type endpoint not available, falling back to charms data');
            const data = await fetchRawCharmsData();
            return countCharmsByType(data.charms || []);
        }

        const counts = await response.json();
        return counts;
    } catch (error) {
        console.error('Error getting charms counts:', error);
        try {
            const data = await fetchRawCharmsData();
            return countCharmsByType(data.charms || []);
        } catch (fallbackError) {
            console.error('Fallback to charms data also failed:', fallbackError);
            return { total: 0, nft: 0, token: 0, dapp: 0 };
        }
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

export const getCharmByTxId = async (txid) => {
    try {
        const response = await fetch(`${ENDPOINTS.CHARM_BY_TXID(txid)}`);

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const data = await response.json();
        return data;
    } catch (error) {
        console.error('[API] Error fetching charm by txid:', error);
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
        console.error('Error liking charm:', error);
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
        console.error('Error unliking charm:', error);
        throw handleApiError(error, 'unlike charm');
    }
};
