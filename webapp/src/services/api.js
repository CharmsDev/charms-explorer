'use client';

// API service for the Charms Explorer
// Re-exports the API services from the modular components

// Re-export all API services
export { fetchAssets, getAssetById, getAssetCounts } from './apiServices';

// Export additional utilities if needed by components
export { paginateItems } from './apiUtils';
export { transformCharmData, transformCharmsArray } from './transformers';
