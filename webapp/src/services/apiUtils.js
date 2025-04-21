'use client';

// Utility functions for API operations

// Attempts to fix malformed JSON by adding missing commas and structure
export const fixJsonResponse = (responseText) => {
    // Fix the JSON by adding commas between properties and wrapping in proper structure
    let fixedJson = responseText;

    // Check if the response starts with 'charms":[' and fix it
    if (responseText.startsWith('charms":[')) {
        fixedJson = '{"' + fixedJson;
    }

    // Add commas between properties
    fixedJson = fixedJson
        .replace(/}{"txid"/g, '},{"txid"')
        .replace(/}"charmid"/g, ',"charmid"')
        .replace(/}"block_height"/g, ',"block_height"')
        .replace(/}"data"/g, ',"data"')
        .replace(/}"date_created"/g, ',"date_created"')
        .replace(/}"asset_type"/g, ',"asset_type"');

    return fixedJson;
};

// Safely extracts a property from an object with nested fallbacks
export const getNestedProperty = (obj, path, defaultValue = undefined) => {
    if (!obj || !path) return defaultValue;

    const keys = path.split('.');
    let current = obj;

    for (const key of keys) {
        if (current === null || current === undefined || typeof current !== 'object') {
            return defaultValue;
        }
        current = current[key];
    }

    return current !== undefined ? current : defaultValue;
};

// Implements pagination for an array of items
export const paginateItems = (items, page, limit) => {
    const startIndex = (page - 1) * limit;
    const paginatedData = items.slice(startIndex, startIndex + limit);

    return {
        data: paginatedData,
        pagination: {
            total: items.length,
            page,
            limit,
            totalPages: Math.ceil(items.length / limit)
        }
    };
};

// Handles API errors consistently
export const handleApiError = (error, context) => {
    console.error(`Error in ${context}:`, error);
    const enhancedError = new Error(`Failed to ${context}`);
    enhancedError.originalError = error;
    return enhancedError;
};
