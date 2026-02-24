'use client';

// API Services - Re-exports from domain-specific modules for backward compatibility
// New code should import directly from './api/charms', './api/assets', or './api/status'

export {
    fetchAssets,
    getCharmsCountByType,
    fetchCharmsByAddress,
    getCharmByTxId,
    likeCharm,
    unlikeCharm
} from './api/charms';

export {
    fetchAssetHolders
} from './api/assets';

export {
    fetchIndexerStatus,
} from './api/status';

export {
    fetchTransactions,
} from './api/transactions';
