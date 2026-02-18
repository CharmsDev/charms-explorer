'use client';

// API service for the Charms Explorer
// Re-exports the API services from the modular components

// Re-export all API services
export { fetchAssets, getCharmsCountByType, fetchIndexerStatus, resetIndexer, likeCharm, unlikeCharm, fetchCharmsByAddress } from './apiServices';

// Re-export from api/ directory modules
export { fetchAssetsByType, getAssetById, getAssetCounts, fetchAssetByAppId, fetchAssetHolders } from './api/assets';
export { fetchNftReferenceMetadata, getRefNftAppId } from './api/tokenMetadata';

// Export utilities used by components
export { transformCharmData, transformCharmsArray } from './transformers';
