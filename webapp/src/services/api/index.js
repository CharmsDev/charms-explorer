'use client';

// Re-export all API functions from domain-specific modules
export {
    fetchRawCharmsData,
    fetchAssets,
    getCharmsCountByType,
    fetchCharmsByAddress,
    getCharmByTxId,
    likeCharm,
    unlikeCharm
} from './charms';

export {
    fetchAssetsByType,
    getAssetById,
    getAssetCounts,
    fetchAssetHolders
} from './assets';

export {
    fetchIndexerStatus,
    resetIndexer
} from './status';
