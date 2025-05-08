'use client';

// API configuration and endpoints

// Base API URL
export const API_BASE_URL = process.env.NEXT_PUBLIC_CHARMS_API_URL || 'http://localhost:5002';

// API Endpoints
export const ENDPOINTS = {
    CHARMS: `${API_BASE_URL}/charms`,
    CHARMS_BY_TYPE: (type) => `${API_BASE_URL}/charms/by-type?type=${encodeURIComponent(type)}`,
    STATUS: `${API_BASE_URL}/status`,
    RESET: `${API_BASE_URL}/reset`,
};
