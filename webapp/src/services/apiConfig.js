'use client';

// API configuration and endpoints

// Base API URL
export const API_BASE_URL = process.env.NEXT_PUBLIC_CHARMS_API_URL || 'http://localhost:8000';

// API Endpoints
export const ENDPOINTS = {
    CHARMS: `${API_BASE_URL}/charms`,
    CHARM_BY_CHARMID: (charmid) => `${API_BASE_URL}/charms/by-charmid/${charmid}`,
    CHARMS_BY_ADDRESS: (address) => `${API_BASE_URL}/charms/by-address/${address}`, // [RJJ-ADDRESS-SEARCH]
    CHARMS_COUNT_BY_TYPE: `${API_BASE_URL}/charms/count-by-type`,
    ASSETS: `${API_BASE_URL}/assets`,
    ASSET_COUNTS: `${API_BASE_URL}/assets/count`,
    ASSET_HOLDERS: (appId) => `${API_BASE_URL}/assets/${encodeURIComponent(appId)}/holders`, // [RJJ-STATS-HOLDERS]
    STATUS: `${API_BASE_URL}/status`,
    RESET: `${API_BASE_URL}/reset`,
    LIKE_CHARM: `${API_BASE_URL}/charms/like`,

    // Helper function to build paginated endpoints
    buildPaginatedUrl: (baseUrl, page = 1, limit = 12, sort = 'newest') => {
        const pageNum = parseInt(page);
        const limitNum = parseInt(limit);
        return `${baseUrl}?page=${pageNum}&limit=${limitNum}&sort=${encodeURIComponent(sort)}`;
    }
};
