/**
 * Charm/Transaction Classifier for Explorer
 * Classifies charms and transactions into different types based on their characteristics
 */

// Charm types based on tags and app_id patterns
export const CHARM_TYPES = {
    BRO_TOKEN: 'bro_token',
    CHARMS_CAST_DEX: 'charms_cast_dex',
    DEX_ORDER: 'dex_order',
    NFT: 'nft',
    TOKEN: 'token',
    DAPP: 'dapp',
    OTHER: 'other'
};

// Labels for UI display
export const CHARM_LABELS = {
    [CHARM_TYPES.BRO_TOKEN]: '$BRO Token',
    [CHARM_TYPES.CHARMS_CAST_DEX]: 'Charms Cast DEX',
    [CHARM_TYPES.DEX_ORDER]: 'DEX Order',
    [CHARM_TYPES.NFT]: 'NFT',
    [CHARM_TYPES.TOKEN]: 'Token',
    [CHARM_TYPES.DAPP]: 'dApp',
    [CHARM_TYPES.OTHER]: 'Other'
};

// Icons/emojis for each type
export const CHARM_ICONS = {
    [CHARM_TYPES.BRO_TOKEN]: '🪙',
    [CHARM_TYPES.CHARMS_CAST_DEX]: '🔄',
    [CHARM_TYPES.DEX_ORDER]: '📊',
    [CHARM_TYPES.NFT]: '🎨',
    [CHARM_TYPES.TOKEN]: '💰',
    [CHARM_TYPES.DAPP]: '⚙️',
    [CHARM_TYPES.OTHER]: '📦'
};

// Color schemes for each type
export const CHARM_COLORS = {
    [CHARM_TYPES.BRO_TOKEN]: {
        bg: 'bg-yellow-500/20',
        text: 'text-yellow-400',
        border: 'border-yellow-500/30',
        gradient: 'from-yellow-400 to-orange-500'
    },
    [CHARM_TYPES.CHARMS_CAST_DEX]: {
        bg: 'bg-purple-500/20',
        text: 'text-purple-400',
        border: 'border-purple-500/30',
        gradient: 'from-purple-400 to-pink-500'
    },
    [CHARM_TYPES.DEX_ORDER]: {
        bg: 'bg-green-500/20',
        text: 'text-green-400',
        border: 'border-green-500/30',
        gradient: 'from-green-400 to-emerald-500'
    },
    [CHARM_TYPES.NFT]: {
        bg: 'bg-pink-500/20',
        text: 'text-pink-400',
        border: 'border-pink-500/30',
        gradient: 'from-pink-400 to-rose-500'
    },
    [CHARM_TYPES.TOKEN]: {
        bg: 'bg-blue-500/20',
        text: 'text-blue-400',
        border: 'border-blue-500/30',
        gradient: 'from-blue-400 to-cyan-500'
    },
    [CHARM_TYPES.DAPP]: {
        bg: 'bg-cyan-500/20',
        text: 'text-cyan-400',
        border: 'border-cyan-500/30',
        gradient: 'from-cyan-400 to-teal-500'
    },
    [CHARM_TYPES.OTHER]: {
        bg: 'bg-gray-500/20',
        text: 'text-gray-400',
        border: 'border-gray-500/30',
        gradient: 'from-gray-400 to-slate-500'
    }
};

// Known BRO token app_id patterns
const BRO_APP_ID_PATTERNS = [
    /^t\/bro$/i,
    /^t\/\$bro$/i,
    /bro/i
];

// Known Charms Cast DEX verification keys
const CHARMS_CAST_VK_PATTERNS = [
    /^b\//i  // Charms Cast orders start with b/
];

/**
 * Classify a charm based on its properties
 * @param {Object} charm - The charm object from API
 * @returns {string} - The charm type
 */
export function classifyCharm(charm) {
    if (!charm) return CHARM_TYPES.OTHER;

    const { tags, app_id, asset_type, name, data } = charm;

    // Check tags first (most reliable)
    if (tags) {
        const tagLower = typeof tags === 'string' ? tags.toLowerCase() : '';
        if (tagLower.includes('charms-cast') || tagLower.includes('dex')) {
            return CHARM_TYPES.CHARMS_CAST_DEX;
        }
        if (tagLower.includes('bro')) {
            return CHARM_TYPES.BRO_TOKEN;
        }
    }

    // Check for DEX data in the charm's data field (from indexer)
    if (data) {
        // Check for charms-cast tag in native_data
        const nativeData = data?.native_data || data?.data?.native_data;
        if (nativeData) {
            // Check app_public_inputs for DEX app (b/ prefix)
            const appInputs = nativeData?.app_public_inputs;
            if (appInputs) {
                const appInputsStr = JSON.stringify(appInputs);
                if (appInputsStr.includes('"b/')) {
                    return CHARM_TYPES.CHARMS_CAST_DEX;
                }
            }
        }
        
        // Check for tags in data
        const dataTags = data?.tags;
        if (dataTags) {
            const tagsStr = typeof dataTags === 'string' ? dataTags : JSON.stringify(dataTags);
            if (tagsStr.toLowerCase().includes('charms-cast')) {
                return CHARM_TYPES.CHARMS_CAST_DEX;
            }
        }
    }

    // Check app_id patterns
    if (app_id) {
        // Charms Cast DEX orders
        if (CHARMS_CAST_VK_PATTERNS.some(pattern => pattern.test(app_id))) {
            return CHARM_TYPES.CHARMS_CAST_DEX;
        }
        
        // BRO token
        if (BRO_APP_ID_PATTERNS.some(pattern => pattern.test(app_id))) {
            return CHARM_TYPES.BRO_TOKEN;
        }
    }

    // Check name for BRO
    if (name && /bro/i.test(name)) {
        return CHARM_TYPES.BRO_TOKEN;
    }

    // Fall back to asset_type
    switch (asset_type?.toLowerCase()) {
        case 'nft':
            return CHARM_TYPES.NFT;
        case 'token':
            return CHARM_TYPES.TOKEN;
        case 'dapp':
            return CHARM_TYPES.DAPP;
        default:
            return CHARM_TYPES.OTHER;
    }
}

/**
 * Get display label for a charm type
 */
export function getCharmLabel(charmType) {
    return CHARM_LABELS[charmType] || CHARM_LABELS[CHARM_TYPES.OTHER];
}

/**
 * Get icon for a charm type
 */
export function getCharmIcon(charmType) {
    return CHARM_ICONS[charmType] || CHARM_ICONS[CHARM_TYPES.OTHER];
}

/**
 * Get color scheme for a charm type
 */
export function getCharmColors(charmType) {
    return CHARM_COLORS[charmType] || CHARM_COLORS[CHARM_TYPES.OTHER];
}

/**
 * Get badge component props for a charm
 */
export function getCharmBadgeProps(charm) {
    const type = classifyCharm(charm);
    const colors = getCharmColors(type);
    const label = getCharmLabel(type);
    const icon = getCharmIcon(type);

    return {
        type,
        label,
        icon,
        className: `${colors.bg} ${colors.text} ${colors.border}`,
        gradientClass: `bg-gradient-to-r ${colors.gradient}`
    };
}

/**
 * Format satoshi amount to BTC string
 */
export function formatBTC(sats) {
    if (!sats && sats !== 0) return '0';
    return (sats / 100000000).toFixed(8);
}

/**
 * Format large numbers with commas
 */
export function formatNumber(num) {
    if (!num && num !== 0) return '0';
    return num.toLocaleString();
}

/**
 * Truncate string in the middle (for txids, addresses)
 */
export function truncateMiddle(str, startChars = 8, endChars = 8) {
    if (!str) return '';
    if (str.length <= startChars + endChars + 3) return str;
    return `${str.slice(0, startChars)}...${str.slice(-endChars)}`;
}
