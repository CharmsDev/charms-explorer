/**
 * QuickNode Bitcoin RPC Service
 * Provides direct access to Bitcoin blockchain data via QuickNode
 */

const QUICKNODE_MAINNET_URL = process.env.NEXT_PUBLIC_QUICKNODE_BITCOIN_MAINNET_URL;
const QUICKNODE_MAINNET_API_KEY = process.env.NEXT_PUBLIC_QUICKNODE_BITCOIN_MAINNET_API_KEY;

/**
 * Make a JSON-RPC call to QuickNode
 */
async function rpcCall(method, params = []) {
    if (!QUICKNODE_MAINNET_URL) {
        throw new Error('QuickNode URL not configured. Set NEXT_PUBLIC_QUICKNODE_BITCOIN_MAINNET_URL');
    }

    const headers = {
        'Content-Type': 'application/json',
    };

    // Add API key if configured
    if (QUICKNODE_MAINNET_API_KEY) {
        headers['x-api-key'] = QUICKNODE_MAINNET_API_KEY;
    }

    const response = await fetch(QUICKNODE_MAINNET_URL, {
        method: 'POST',
        headers,
        body: JSON.stringify({
            jsonrpc: '2.0',
            id: Date.now(),
            method,
            params,
        }),
    });

    if (!response.ok) {
        throw new Error(`QuickNode RPC error: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();

    if (data.error) {
        throw new Error(`RPC error: ${data.error.message}`);
    }

    return data.result;
}

/**
 * Get raw transaction data
 * @param {string} txid - Transaction ID
 * @param {boolean} verbose - If true, returns decoded transaction
 * @returns {Promise<Object>} Transaction data
 */
export async function getRawTransaction(txid, verbose = true) {
    return await rpcCall('getrawtransaction', [txid, verbose]);
}

/**
 * Get transaction with block info
 * @param {string} txid - Transaction ID
 * @returns {Promise<Object>} Transaction with block data
 */
export async function getTransaction(txid) {
    try {
        const tx = await getRawTransaction(txid, true);
        
        // Transform to format similar to Mempool.space for compatibility
        return {
            txid: tx.txid,
            version: tx.version,
            locktime: tx.locktime,
            size: tx.size,
            weight: tx.weight,
            fee: tx.fee ? Math.round(tx.fee * 100000000) : null, // Convert BTC to sats
            vin: tx.vin.map((input, idx) => ({
                txid: input.txid,
                vout: input.vout,
                sequence: input.sequence,
                witness: input.txinwitness || [],
                prevout: input.prevout ? {
                    value: Math.round(input.prevout.value * 100000000), // Convert BTC to sats
                    scriptpubkey: input.prevout.scriptPubKey?.hex || '',
                    scriptpubkey_address: input.prevout.scriptPubKey?.address || null,
                } : null,
            })),
            vout: tx.vout.map((output) => ({
                value: Math.round(output.value * 100000000), // Convert BTC to sats
                n: output.n,
                scriptpubkey: output.scriptPubKey?.hex || '',
                scriptpubkey_address: output.scriptPubKey?.address || null,
                scriptpubkey_type: output.scriptPubKey?.type || null,
            })),
            status: {
                confirmed: !!tx.confirmations,
                block_height: tx.blockheight || null,
                block_hash: tx.blockhash || null,
                block_time: tx.blocktime || null,
            },
        };
    } catch (error) {
        // Silently fail in production, throw in dev
        if (process.env.NODE_ENV === 'development') {
            console.error('[QuickNode] Error fetching transaction:', error);
        }
        throw error;
    }
}

/**
 * Get block by hash
 * @param {string} blockhash - Block hash
 * @param {number} verbosity - 0=hex, 1=json, 2=json with tx data
 * @returns {Promise<Object>} Block data
 */
export async function getBlock(blockhash, verbosity = 1) {
    return await rpcCall('getblock', [blockhash, verbosity]);
}

/**
 * Get blockchain info
 * @returns {Promise<Object>} Blockchain information
 */
export async function getBlockchainInfo() {
    return await rpcCall('getblockchaininfo', []);
}

/**
 * Check if QuickNode is configured
 * @returns {boolean} True if QuickNode is available
 */
export function isQuickNodeAvailable() {
    return !!(QUICKNODE_MAINNET_URL && QUICKNODE_MAINNET_URL.trim() !== '');
}

export default {
    getRawTransaction,
    getTransaction,
    getBlock,
    getBlockchainInfo,
    isQuickNodeAvailable,
};
