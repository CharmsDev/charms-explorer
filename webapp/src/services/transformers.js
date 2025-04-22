'use client';

import { getNestedProperty } from './apiUtils';

// Data transformation functions for API responses

// Detects the type of a charm based on its data
export const detectCharmType = (charm) => {
    const apps = getNestedProperty(charm, 'data.data.apps');
    if (apps && typeof apps === 'object') {
        for (const key in apps) {
            const appValue = apps[key];
            if (typeof appValue === 'string') {
                if (appValue.startsWith('n/')) {
                    return 'nft';
                }
                if (appValue.startsWith('t/')) {
                    return 'token';
                }
                // Add rules for 'dapp' if they become available
            }
        }
    }
    // Fallback or default type if no specific type detected
    // For now, let's keep the original asset_type if detection fails,
    // or default to 'unknown' if asset_type is also missing.
    return charm?.asset_type || 'unknown';
};


// Transforms a charm object from the API to the format expected by the UI
export const transformCharmData = (charm) => {
    if (!charm) return null;

    const detectedType = detectCharmType(charm);

    const name = getNestedProperty(charm, 'data.data.name') ||
        `Charm ${charm.charmid?.substring(0, 8) || 'Unknown'}`;

    const description = getNestedProperty(charm, 'data.data.description') ||
        'No description available';

    const image = getNestedProperty(charm, 'data.data.image') ||
        'https://charms.dev/_astro/logo-charms-dark.Ceshk2t3.png';

    const ticker = getNestedProperty(charm, 'data.data.ticker') || '';

    const supply = getNestedProperty(charm, 'data.data.supply') || 0;
    const remaining = getNestedProperty(charm, 'data.data.remaining') || 0;

    const url = getNestedProperty(charm, 'data.data.url') || '';

    const attributes = getNestedProperty(charm, 'data.data.attributes') || [];

    const likes = Math.floor(Math.random() * 100);
    const comments = Math.floor(Math.random() * 20);

    return {
        id: charm.charmid,
        type: detectedType,
        name,
        description,
        image,
        txid: charm.txid,
        outputIndex: 0,
        address: '',
        createdAt: charm.date_created,
        likes,
        comments,
        ticker,
        supply,
        remaining,
        url,
        attributes
    };
};

// Transforms an array of charms from the API
export const transformCharmsArray = (charms) => {
    if (!Array.isArray(charms)) return [];
    return charms.map(transformCharmData).filter(Boolean);
};

// Counts charms by type using the detection logic
export const countCharmsByType = (charms) => {
    if (!Array.isArray(charms)) return { total: 0, nft: 0, token: 0, dapp: 0 };

    let nftCount = 0;
    let tokenCount = 0;
    let dappCount = 0;

    charms.forEach(charm => {
        const type = detectCharmType(charm);
        if (type === 'nft') {
            nftCount++;
        } else if (type === 'token') {
            tokenCount++;
        } else if (type === 'dapp') {
            dappCount++;
        }
    });

    const totalCount = charms.length;

    return {
        total: totalCount,
        nft: nftCount,
        token: tokenCount,
        dapp: dappCount,
    };
};

// Creates a default charm object when no data is available
export const createDefaultCharm = (id) => {
    return {
        id: id,
        type: 'unknown',
        name: `Charm ${id.substring(0, 8)}`,
        description: 'No description available',
        image: 'https://charms.dev/_astro/logo-charms-dark.Ceshk2t3.png',
        txid: '',
        outputIndex: 0,
        address: '',
        createdAt: new Date().toISOString(),
        likes: 0,
        comments: 0,
        ticker: '',
        supply: 0,
        remaining: 0,
        url: '',
        attributes: []
    };
};
