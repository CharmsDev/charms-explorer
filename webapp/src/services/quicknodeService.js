/**
 * [LEGACY SHIM] Re-exports from failover/quicknodeService.js
 * All QuickNode code now lives in services/failover/.
 * This file exists only so existing imports don't break.
 */
export {
    getRawTransaction,
    getTransaction,
    getBlock,
    getBlockchainInfo,
    isQuickNodeAvailable,
} from './failover/quicknodeService';

export { default } from './failover/quicknodeService';
