'use client';

// API configuration and endpoints

// Base API URL (v1 — versioned API)
export const API_BASE_URL = process.env.NEXT_PUBLIC_CHARMS_API_URL || 'http://localhost:8000/v1';

// API Endpoints
export const ENDPOINTS = {
    CHARMS: `${API_BASE_URL}/charms`,
    CHARM_BY_CHARMID: (charmid) => `${API_BASE_URL}/charms/by-charmid/${encodeURIComponent(charmid)}`,
    CHARM_BY_TXID: (txid) => `${API_BASE_URL}/charms/${txid}`,
    CHARMS_BY_ADDRESS: (address) => `${API_BASE_URL}/charms/by-address/${address}`, // [RJJ-ADDRESS-SEARCH]
    CHARMS_COUNT_BY_TYPE: `${API_BASE_URL}/charms/count-by-type`,
    ASSETS: `${API_BASE_URL}/assets`,
    ASSET_COUNTS: `${API_BASE_URL}/assets/count`,
    ASSET_HOLDERS: (appId) => `${API_BASE_URL}/assets/${encodeURIComponent(appId)}/holders`, // [RJJ-STATS-HOLDERS]
    REFERENCE_NFT: (hash) => `${API_BASE_URL}/assets/reference-nft/${encodeURIComponent(hash)}`, // [RJJ-REF-NFT]
    STATUS: `${API_BASE_URL}/status`,
    LIKE_CHARM: `${API_BASE_URL}/charms/like`,

    // DEX endpoints
    DEX_OPEN_ORDERS: `${API_BASE_URL}/dex/orders/open`,
    DEX_ORDERS_BY_ASSET: (assetAppId) => `${API_BASE_URL}/dex/orders/by-asset/${encodeURIComponent(assetAppId)}`,
    DEX_ORDERS_BY_MAKER: (maker) => `${API_BASE_URL}/dex/orders/by-maker/${encodeURIComponent(maker)}`,
    DEX_ORDER_BY_ID: (orderId) => `${API_BASE_URL}/dex/orders/${encodeURIComponent(orderId)}`,

    // Helper function to build paginated endpoints
    buildPaginatedUrl: (baseUrl, page = 1, limit = 12, sort = 'newest') => {
        const pageNum = parseInt(page);
        const limitNum = parseInt(limit);
        return `${baseUrl}?page=${pageNum}&limit=${limitNum}&sort=${encodeURIComponent(sort)}`;
    }
};
