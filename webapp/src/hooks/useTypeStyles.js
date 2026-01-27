'use client';

const TYPE_STYLES = {
    nft: {
        color: 'from-purple-500 to-indigo-700',
        bgColor: 'bg-purple-900/20',
        borderColor: 'border-purple-500/30',
        icon: '🖼️',
        label: 'NFT'
    },
    token: {
        color: 'from-green-500 to-emerald-700',
        bgColor: 'bg-green-900/20',
        borderColor: 'border-green-500/30',
        icon: '🪙',
        label: 'Token'
    },
    dapp: {
        color: 'from-blue-500 to-cyan-700',
        bgColor: 'bg-blue-900/20',
        borderColor: 'border-blue-500/30',
        icon: '⚡',
        label: 'DApp'
    },
    bro_token: {
        color: 'from-yellow-500 to-orange-600',
        bgColor: 'bg-yellow-900/20',
        borderColor: 'border-yellow-500/30',
        icon: '🪙',
        label: '$BRO'
    },
    dex: {
        color: 'from-purple-500 to-pink-600',
        bgColor: 'bg-purple-900/20',
        borderColor: 'border-purple-500/30',
        icon: '🔄',
        label: 'DEX'
    },
    default: {
        color: 'from-gray-500 to-slate-700',
        bgColor: 'bg-gray-900/20',
        borderColor: 'border-gray-500/30',
        icon: '📦',
        label: 'Asset'
    }
};

export function useTypeStyles(assetType, charmType = null) {
    // Handle special charm types first
    if (charmType === 'BRO_TOKEN') {
        return TYPE_STYLES.bro_token;
    }
    if (charmType === 'CHARMS_CAST_DEX' || charmType === 'DEX_ORDER') {
        return TYPE_STYLES.dex;
    }

    // Then handle asset types
    const normalizedType = assetType?.toLowerCase();
    return TYPE_STYLES[normalizedType] || { 
        ...TYPE_STYLES.default, 
        label: assetType || 'Asset' 
    };
}

export function getTypeStyles(assetType, charmType = null) {
    if (charmType === 'BRO_TOKEN') {
        return TYPE_STYLES.bro_token;
    }
    if (charmType === 'CHARMS_CAST_DEX' || charmType === 'DEX_ORDER') {
        return TYPE_STYLES.dex;
    }

    const normalizedType = assetType?.toLowerCase();
    return TYPE_STYLES[normalizedType] || { 
        ...TYPE_STYLES.default, 
        label: assetType || 'Asset' 
    };
}
