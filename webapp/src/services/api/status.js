'use client';

import { ENDPOINTS } from '../apiConfig';
import { handleApiError } from '../apiUtils';

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
