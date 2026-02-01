'use client';

// API service for the Charms Explorer
// Re-exports the API services from the modular components

// Re-export all API services
export { fetchAssets, fetchAssetsByType, getAssetById, getAssetCounts, getCharmsCountByType, fetchIndexerStatus, resetIndexer, likeCharm, unlikeCharm, fetchCharmsByAddress } from './apiServices';

// Re-export from api/ directory modules
export { fetchUniqueAssets, getUniqueAssetCounts, fetchAssetByAppId, fetchAssetHolders } from './api/assets';
export { fetchNftReferenceMetadata, getRefNftAppId } from './api/tokenMetadata';

// Export additional utilities if needed by components
export { paginateItems } from './apiUtils';
export { transformCharmData, transformCharmsArray } from './transformers';
