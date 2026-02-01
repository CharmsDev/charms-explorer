'use client';

// API Services - Re-exports from domain-specific modules for backward compatibility
// New code should import directly from './api/charms', './api/assets', or './api/status'

export {
    fetchRawCharmsData,
    fetchAssets,
    getCharmsCountByType,
    fetchCharmsByAddress,
    getCharmByTxId,
    likeCharm,
    unlikeCharm
} from './api/charms';

export {
    fetchAssetsByType,
    getAssetById,
    getAssetCounts,
    fetchAssetHolders,
    fetchAssetByAppId
} from './api/assets';

export {
    fetchIndexerStatus,
    resetIndexer
} from './api/status';
