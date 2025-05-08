'use client';

import { ENDPOINTS } from './apiConfig';
import { fixJsonResponse, handleApiError, paginateItems } from './apiUtils';
import { transformCharmsArray, countCharmsByType, createDefaultCharm, detectCharmType } from './transformers';

// Core API service functions

// Fetches raw charm data from the API
export const fetchRawCharmsData = async () => {
    try {
        const response = await fetch(ENDPOINTS.CHARMS);

        if (!response.ok) {
            throw new Error(`API error: ${response.status}`);
        }

        // Get the response text
        const responseText = await response.text();
        console.log('Raw API response:', responseText);

        try {
            // Try to parse the JSON directly first
            const data = JSON.parse(responseText);
            console.log('Parsed JSON data:', data);
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
                return data;
            } catch (error) {
                console.error('Error fixing JSON:', error);
                throw new Error('Failed to parse API response');
            }
        }
    } catch (error) {
        throw handleApiError(error, 'fetch charms data');
    }
};

// Fetches and transforms charm assets with pagination
export const fetchAssets = async (type = 'all', page = 1, limit = 20) => {
    try {
        const data = await fetchRawCharmsData();

        // Filter by type using the detection logic if needed
        let filteredCharms = data.charms;
        if (type !== 'all') {
            // Use detectCharmType for filtering
            filteredCharms = data.charms.filter(charm => detectCharmType(charm) === type);
        }

        // Transform the *filtered* data
        const transformedCharms = transformCharmsArray(filteredCharms);

        // Paginate the results
        return paginateItems(transformedCharms, page, limit);
    } catch (error) {
        throw handleApiError(error, 'fetch assets');
    }
};

// Gets a specific asset by ID
export const getAssetById = async (id) => {
    try {
        const data = await fetchRawCharmsData();

        // Find the charm with the matching ID
        const charm = data.charms.find(charm => charm.charmid === id);

        if (!charm) {
            return createDefaultCharm(id);
        }

        // Transform the charm data
        return transformCharmsArray([charm])[0];
    } catch (error) {
        throw handleApiError(error, 'fetch asset details');
    }
};

// Gets counts of assets by type
export const getAssetCounts = async () => {
    try {
        const data = await fetchRawCharmsData();
        return countCharmsByType(data.charms);
    } catch (error) {
        throw handleApiError(error, 'fetch asset counts');
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
