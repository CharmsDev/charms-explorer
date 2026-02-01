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

// Color schemes for each type - Dark mode cohesive palette
export const CHARM_COLORS = {
    [CHARM_TYPES.BRO_TOKEN]: {
        bg: 'bg-amber-500/10',
        text: 'text-amber-400',
        border: 'border-amber-500/20',
        gradient: 'from-amber-400 to-yellow-500'
    },
    [CHARM_TYPES.CHARMS_CAST_DEX]: {
        bg: 'bg-violet-500/10',
        text: 'text-violet-400',
        border: 'border-violet-500/20',
        gradient: 'from-violet-400 to-purple-500'
    },
    [CHARM_TYPES.DEX_ORDER]: {
        bg: 'bg-violet-500/10',
        text: 'text-violet-400',
        border: 'border-violet-500/20',
        gradient: 'from-violet-400 to-purple-500'
    },
    [CHARM_TYPES.NFT]: {
        bg: 'bg-purple-500/10',
        text: 'text-purple-400',
        border: 'border-purple-500/20',
        gradient: 'from-purple-400 to-violet-500'
    },
    [CHARM_TYPES.TOKEN]: {
        bg: 'bg-amber-500/10',
        text: 'text-amber-300',
        border: 'border-amber-500/20',
        gradient: 'from-amber-300 to-orange-400'
    },
    [CHARM_TYPES.DAPP]: {
        bg: 'bg-slate-500/10',
        text: 'text-slate-300',
        border: 'border-slate-500/20',
        gradient: 'from-slate-400 to-slate-500'
    },
    [CHARM_TYPES.OTHER]: {
        bg: 'bg-slate-500/10',
        text: 'text-slate-400',
        border: 'border-slate-500/20',
        gradient: 'from-slate-400 to-slate-500'
    }
};

// VERIFIED BRO token - only this specific app_id hash is the official $BRO
// This is the reference NFT hash that defines the official BRO token
const VERIFIED_BRO_HASH = '3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b';

/**
 * Check if an app_id belongs to the verified BRO token/NFT family
 * Only tokens/NFTs with this specific hash are official $BRO
 */
function isVerifiedBro(app_id) {
    if (!app_id) return false;
    // Match t/HASH/... or n/HASH/... patterns
    return app_id.includes(VERIFIED_BRO_HASH);
}

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

    // Check tags first for DEX detection
    if (tags) {
        const tagLower = typeof tags === 'string' ? tags.toLowerCase() : '';
        if (tagLower.includes('charms-cast') || tagLower.includes('dex')) {
            return CHARM_TYPES.CHARMS_CAST_DEX;
        }
    }

    // Check for VERIFIED BRO token (must have specific hash in app_id)
    if (app_id && isVerifiedBro(app_id)) {
        return CHARM_TYPES.BRO_TOKEN;
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

    // Check app_id patterns for DEX
    if (app_id) {
        // Charms Cast DEX orders
        if (CHARMS_CAST_VK_PATTERNS.some(pattern => pattern.test(app_id))) {
            return CHARM_TYPES.CHARMS_CAST_DEX;
        }
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
