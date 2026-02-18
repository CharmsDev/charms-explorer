'use client';

import { ENDPOINTS } from '../apiConfig';

export const fetchOpenDexOrders = async (network = null) => {
    try {
        let url = ENDPOINTS.DEX_OPEN_ORDERS;
        if (network && network !== 'all') {
            url += `?network=${encodeURIComponent(network)}`;
        }
        
        const response = await fetch(url);
        
        if (!response.ok) {
            throw new Error(`API error: ${response.status}`);
        }
        
        const data = await response.json();
        return data;
    } catch (error) {
        console.error('Failed to fetch DEX orders:', error);
        return { orders: [] };
    }
};

export const fetchDexOrdersByAsset = async (assetAppId) => {
    try {
        const response = await fetch(ENDPOINTS.DEX_ORDERS_BY_ASSET(assetAppId));
        
        if (!response.ok) {
            throw new Error(`API error: ${response.status}`);
        }
        
        return await response.json();
    } catch (error) {
        console.error('Failed to fetch DEX orders by asset:', error);
        return { orders: [] };
    }
};

export const fetchDexOrdersByMaker = async (maker) => {
    try {
        const response = await fetch(ENDPOINTS.DEX_ORDERS_BY_MAKER(maker));
        
        if (!response.ok) {
            throw new Error(`API error: ${response.status}`);
        }
        
        return await response.json();
    } catch (error) {
        console.error('Failed to fetch DEX orders by maker:', error);
        return { orders: [] };
    }
};
