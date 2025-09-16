'use client';

// API configuration and endpoints

// Base API URL
export const API_BASE_URL = process.env.NEXT_PUBLIC_CHARMS_API_URL || 'http://localhost:8000';

// API Endpoints
export const ENDPOINTS = {
    CHARMS: `${API_BASE_URL}/charms`,
    CHARMS_BY_TYPE: (type) => `${API_BASE_URL}/charms/by-type?type=${encodeURIComponent(type)}`,
    CHARM_BY_CHARMID: (charmid) => `${API_BASE_URL}/charms/by-charmid/${encodeURIComponent(charmid)}`,
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
