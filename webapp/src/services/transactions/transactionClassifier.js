/**
 * Transaction Classifier Service
 * Classifies blockchain transactions into different types based on their characteristics
 * Modular architecture designed to scale with new transaction types
 */

// ============================================================================
// TRANSACTION TYPES
// ============================================================================

export const TRANSACTION_TYPES = {
    // Bitcoin transactions
    BITCOIN_TRANSFER: 'bitcoin_transfer',
    
    // Charm/Token transactions
    TOKEN_MINT: 'token_mint',
    TOKEN_TRANSFER: 'token_transfer',
    TOKEN_BURN: 'token_burn',
    
    // NFT transactions
    NFT_MINT: 'nft_mint',
    NFT_TRANSFER: 'nft_transfer',
    
    // DEX transactions
    DEX_CREATE_ASK: 'dex_create_ask',
    DEX_CREATE_BID: 'dex_create_bid',
    DEX_FULFILL_ASK: 'dex_fulfill_ask',
    DEX_FULFILL_BID: 'dex_fulfill_bid',
    DEX_CANCEL: 'dex_cancel',
    DEX_PARTIAL_FILL: 'dex_partial_fill',
    
    // BRO specific
    BRO_MINING: 'bro_mining',
    BRO_MINT: 'bro_mint',
    
    // Generic
    SPELL: 'spell',
    UNKNOWN: 'unknown'
};

// ============================================================================
// TRANSACTION METADATA
// ============================================================================

export const TRANSACTION_METADATA = {
    [TRANSACTION_TYPES.BITCOIN_TRANSFER]: {
        label: 'Bitcoin Transfer',
        icon: '₿',
        color: 'orange',
        bgClass: 'bg-orange-500/20',
        textClass: 'text-orange-400',
        borderClass: 'border-orange-500/30',
        description: 'Standard Bitcoin transaction'
    },
    [TRANSACTION_TYPES.TOKEN_MINT]: {
        label: 'Token Mint',
        icon: '🪙',
        color: 'purple',
        bgClass: 'bg-purple-500/20',
        textClass: 'text-purple-400',
        borderClass: 'border-purple-500/30',
        description: 'New tokens created'
    },
    [TRANSACTION_TYPES.TOKEN_TRANSFER]: {
        label: 'Token Transfer',
        icon: '↔️',
        color: 'blue',
        bgClass: 'bg-blue-500/20',
        textClass: 'text-blue-400',
        borderClass: 'border-blue-500/30',
        description: 'Tokens transferred between addresses'
    },
    [TRANSACTION_TYPES.TOKEN_BURN]: {
        label: 'Token Burn',
        icon: '🔥',
        color: 'red',
        bgClass: 'bg-red-500/20',
        textClass: 'text-red-400',
        borderClass: 'border-red-500/30',
        description: 'Tokens permanently destroyed'
    },
    [TRANSACTION_TYPES.NFT_MINT]: {
        label: 'NFT Mint',
        icon: '🎨',
        color: 'pink',
        bgClass: 'bg-pink-500/20',
        textClass: 'text-pink-400',
        borderClass: 'border-pink-500/30',
        description: 'New NFT created'
    },
    [TRANSACTION_TYPES.NFT_TRANSFER]: {
        label: 'NFT Transfer',
        icon: '🖼️',
        color: 'indigo',
        bgClass: 'bg-indigo-500/20',
        textClass: 'text-indigo-400',
        borderClass: 'border-indigo-500/30',
        description: 'NFT transferred to new owner'
    },
    [TRANSACTION_TYPES.DEX_CREATE_ASK]: {
        label: 'DEX Ask Order',
        icon: '📈',
        color: 'green',
        bgClass: 'bg-green-500/20',
        textClass: 'text-green-400',
        borderClass: 'border-green-500/30',
        description: 'Sell order created on DEX'
    },
    [TRANSACTION_TYPES.DEX_CREATE_BID]: {
        label: 'DEX Bid Order',
        icon: '📉',
        color: 'blue',
        bgClass: 'bg-blue-500/20',
        textClass: 'text-blue-400',
        borderClass: 'border-blue-500/30',
        description: 'Buy order created on DEX'
    },
    [TRANSACTION_TYPES.DEX_FULFILL_ASK]: {
        label: 'DEX Fulfill Ask',
        icon: '✅',
        color: 'emerald',
        bgClass: 'bg-emerald-500/20',
        textClass: 'text-emerald-400',
        borderClass: 'border-emerald-500/30',
        description: 'Sell order executed'
    },
    [TRANSACTION_TYPES.DEX_FULFILL_BID]: {
        label: 'DEX Fulfill Bid',
        icon: '✅',
        color: 'emerald',
        bgClass: 'bg-emerald-500/20',
        textClass: 'text-emerald-400',
        borderClass: 'border-emerald-500/30',
        description: 'Buy order executed'
    },
    [TRANSACTION_TYPES.DEX_CANCEL]: {
        label: 'DEX Cancel',
        icon: '❌',
        color: 'red',
        bgClass: 'bg-red-500/20',
        textClass: 'text-red-400',
        borderClass: 'border-red-500/30',
        description: 'Order cancelled'
    },
    [TRANSACTION_TYPES.DEX_PARTIAL_FILL]: {
        label: 'DEX Partial Fill',
        icon: '⚡',
        color: 'yellow',
        bgClass: 'bg-yellow-500/20',
        textClass: 'text-yellow-400',
        borderClass: 'border-yellow-500/30',
        description: 'Order partially filled'
    },
    [TRANSACTION_TYPES.BRO_MINING]: {
        label: 'BRO Mining',
        icon: '⛏️',
        color: 'orange',
        bgClass: 'bg-orange-500/20',
        textClass: 'text-orange-400',
        borderClass: 'border-orange-500/30',
        description: 'BRO token mining transaction'
    },
    [TRANSACTION_TYPES.BRO_MINT]: {
        label: 'BRO Mint',
        icon: '🪙',
        color: 'orange',
        bgClass: 'bg-orange-500/20',
        textClass: 'text-orange-400',
        borderClass: 'border-orange-500/30',
        description: 'BRO token minted'
    },
    [TRANSACTION_TYPES.SPELL]: {
        label: 'Spell',
        icon: '✨',
        color: 'purple',
        bgClass: 'bg-purple-500/20',
        textClass: 'text-purple-400',
        borderClass: 'border-purple-500/30',
        description: 'Charms spell transaction'
    },
    [TRANSACTION_TYPES.UNKNOWN]: {
        label: 'Unknown',
        icon: '❓',
        color: 'gray',
        bgClass: 'bg-gray-500/20',
        textClass: 'text-gray-400',
        borderClass: 'border-gray-500/30',
        description: 'Unknown transaction type'
    }
};

// ============================================================================
// CLASSIFICATION RULES (Modular - easy to add new rules)
// ============================================================================

/**
 * Classification rule interface:
 * {
 *   name: string,
 *   priority: number (lower = higher priority),
 *   test: (tx, spellData) => boolean,
 *   type: TRANSACTION_TYPES value
 * }
 */

const classificationRules = [
    // DEX Rules (highest priority - check tags first)
    {
        name: 'DEX Create Ask',
        priority: 10,
        test: (tx, spellData) => {
            const tags = tx.tags || '';
            return tags.includes('create-ask') || 
                   (spellData?.side === 'ask' && !tags.includes('fulfill'));
        },
        type: TRANSACTION_TYPES.DEX_CREATE_ASK
    },
    {
        name: 'DEX Create Bid',
        priority: 10,
        test: (tx, spellData) => {
            const tags = tx.tags || '';
            return tags.includes('create-bid') || 
                   (spellData?.side === 'bid' && !tags.includes('fulfill'));
        },
        type: TRANSACTION_TYPES.DEX_CREATE_BID
    },
    {
        name: 'DEX Fulfill Ask',
        priority: 10,
        test: (tx) => {
            const tags = tx.tags || '';
            return tags.includes('fulfill-ask');
        },
        type: TRANSACTION_TYPES.DEX_FULFILL_ASK
    },
    {
        name: 'DEX Fulfill Bid',
        priority: 10,
        test: (tx) => {
            const tags = tx.tags || '';
            return tags.includes('fulfill-bid');
        },
        type: TRANSACTION_TYPES.DEX_FULFILL_BID
    },
    {
        name: 'DEX Cancel',
        priority: 10,
        test: (tx) => {
            const tags = tx.tags || '';
            return tags.includes('cancel');
        },
        type: TRANSACTION_TYPES.DEX_CANCEL
    },
    {
        name: 'DEX Partial Fill',
        priority: 10,
        test: (tx) => {
            const tags = tx.tags || '';
            return tags.includes('partial-fill');
        },
        type: TRANSACTION_TYPES.DEX_PARTIAL_FILL
    },
    
    // BRO specific rules
    {
        name: 'BRO Mining',
        priority: 20,
        test: (tx, spellData, rawData) => {
            // Check for OP_RETURN at index 0 with 333 or 777 sats
            if (rawData?.vout) {
                const hasOpReturn = rawData.vout[0]?.scriptpubkey_type === 'op_return';
                const has333or777 = rawData.vout.some(o => o.value === 333 || o.value === 777);
                return hasOpReturn && has333or777;
            }
            return false;
        },
        type: TRANSACTION_TYPES.BRO_MINING
    },
    {
        name: 'BRO Mint',
        priority: 20,
        test: (tx, spellData, rawData) => {
            // Check for charm output with 330 or 1000 sats (typical mint amounts)
            if (rawData?.vout) {
                return rawData.vout.some(o => o.value === 330 || o.value === 1000);
            }
            return false;
        },
        type: TRANSACTION_TYPES.BRO_MINT
    },
    
    // NFT rules
    {
        name: 'NFT Mint',
        priority: 30,
        test: (tx) => {
            const appId = tx.app_id || tx.charmid || '';
            return appId.startsWith('n/') && tx.asset_type === 'nft';
        },
        type: TRANSACTION_TYPES.NFT_MINT
    },
    {
        name: 'NFT Transfer',
        priority: 30,
        test: (tx, spellData) => {
            const appId = tx.app_id || tx.charmid || '';
            if (!appId.startsWith('n/')) return false;
            // Check if there are multiple inputs (transfer vs mint)
            const ins = spellData?.tx?.ins || [];
            return ins.length > 0;
        },
        type: TRANSACTION_TYPES.NFT_TRANSFER
    },
    
    // Token rules
    {
        name: 'Token Mint',
        priority: 40,
        test: (tx, spellData) => {
            const appId = tx.app_id || tx.charmid || '';
            if (!appId.startsWith('t/')) return false;
            // Mint typically has no charm inputs
            const ins = spellData?.tx?.ins || [];
            return ins.length === 0 || !spellData;
        },
        type: TRANSACTION_TYPES.TOKEN_MINT
    },
    {
        name: 'Token Transfer',
        priority: 40,
        test: (tx, spellData) => {
            const appId = tx.app_id || tx.charmid || '';
            if (!appId.startsWith('t/')) return false;
            // Transfer has charm inputs
            const ins = spellData?.tx?.ins || [];
            return ins.length > 0;
        },
        type: TRANSACTION_TYPES.TOKEN_TRANSFER
    },
    
    // Generic spell
    {
        name: 'Spell',
        priority: 50,
        test: (tx, spellData) => {
            return spellData?.detected === true || spellData?.has_native_data === true;
        },
        type: TRANSACTION_TYPES.SPELL
    },
    
    // Bitcoin transfer (lowest priority - fallback)
    {
        name: 'Bitcoin Transfer',
        priority: 100,
        test: (tx) => {
            return tx.isBitcoinTx === true || tx.asset_type === 'bitcoin';
        },
        type: TRANSACTION_TYPES.BITCOIN_TRANSFER
    }
];

// Sort rules by priority
const sortedRules = [...classificationRules].sort((a, b) => a.priority - b.priority);

// ============================================================================
// CLASSIFICATION FUNCTIONS
// ============================================================================

/**
 * Extract spell data from transaction
 */
export function extractSpellData(tx) {
    const data = tx.data?.native_data || tx.native_data || tx.data;
    if (!data) return null;
    
    // Extract order details from DEX transactions
    const outs = data.tx?.outs || [];
    for (const out of outs) {
        for (const [key, value] of Object.entries(out)) {
            if (value && typeof value === 'object' && value.side) {
                return {
                    ...data,
                    orderDetails: {
                        side: value.side,
                        amount: value.amount,
                        quantity: value.quantity,
                        price: value.price,
                        maker: value.maker,
                        asset: value.asset?.token
                    }
                };
            }
        }
    }
    
    return data;
}

/**
 * Classify a transaction into a specific type
 * @param {Object} tx - Transaction object from API
 * @param {Object} rawData - Optional raw Bitcoin transaction data
 * @returns {string} Transaction type from TRANSACTION_TYPES
 */
export function classifyTransaction(tx, rawData = null) {
    if (!tx) return TRANSACTION_TYPES.UNKNOWN;
    
    const spellData = extractSpellData(tx);
    
    // Run through classification rules in priority order
    for (const rule of sortedRules) {
        try {
            if (rule.test(tx, spellData, rawData)) {
                return rule.type;
            }
        } catch (error) {
            console.warn(`[TransactionClassifier] Rule "${rule.name}" failed:`, error);
        }
    }
    
    return TRANSACTION_TYPES.UNKNOWN;
}

/**
 * Get metadata for a transaction type
 */
export function getTransactionMetadata(type) {
    return TRANSACTION_METADATA[type] || TRANSACTION_METADATA[TRANSACTION_TYPES.UNKNOWN];
}

/**
 * Get label for a transaction type
 */
export function getTransactionLabel(type) {
    return getTransactionMetadata(type).label;
}

/**
 * Get icon for a transaction type
 */
export function getTransactionIcon(type) {
    return getTransactionMetadata(type).icon;
}

/**
 * Get color classes for a transaction type
 */
export function getTransactionColors(type) {
    const meta = getTransactionMetadata(type);
    return {
        bg: meta.bgClass,
        text: meta.textClass,
        border: meta.borderClass
    };
}

/**
 * Check if transaction is a DEX transaction
 */
export function isDexTransaction(type) {
    return [
        TRANSACTION_TYPES.DEX_CREATE_ASK,
        TRANSACTION_TYPES.DEX_CREATE_BID,
        TRANSACTION_TYPES.DEX_FULFILL_ASK,
        TRANSACTION_TYPES.DEX_FULFILL_BID,
        TRANSACTION_TYPES.DEX_CANCEL,
        TRANSACTION_TYPES.DEX_PARTIAL_FILL
    ].includes(type);
}

/**
 * Check if transaction is a token transaction
 */
export function isTokenTransaction(type) {
    return [
        TRANSACTION_TYPES.TOKEN_MINT,
        TRANSACTION_TYPES.TOKEN_TRANSFER,
        TRANSACTION_TYPES.TOKEN_BURN,
        TRANSACTION_TYPES.BRO_MINING,
        TRANSACTION_TYPES.BRO_MINT
    ].includes(type);
}

/**
 * Check if transaction is an NFT transaction
 */
export function isNftTransaction(type) {
    return [
        TRANSACTION_TYPES.NFT_MINT,
        TRANSACTION_TYPES.NFT_TRANSFER
    ].includes(type);
}

/**
 * Full transaction analysis - returns all relevant information
 */
export function analyzeTransaction(tx, rawData = null) {
    const type = classifyTransaction(tx, rawData);
    const metadata = getTransactionMetadata(type);
    const spellData = extractSpellData(tx);
    
    return {
        type,
        metadata,
        spellData,
        isDex: isDexTransaction(type),
        isToken: isTokenTransaction(type),
        isNft: isNftTransaction(type),
        isBitcoin: type === TRANSACTION_TYPES.BITCOIN_TRANSFER,
        orderDetails: spellData?.orderDetails || null
    };
}
