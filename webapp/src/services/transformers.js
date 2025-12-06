'use client';

import { getNestedProperty } from './apiUtils';

// Data transformation functions for API responses

// Detects the type of a charm based on its data
export const detectCharmType = (charm) => {
    // First, check if asset_type is directly provided (from API)
    if (charm?.asset_type) {
        return charm.asset_type;
    }

    // Check app_id prefix (most reliable method)
    if (charm?.app_id) {
        if (charm.app_id.startsWith('n/')) {
            return 'nft';
        }
        if (charm.app_id.startsWith('t/')) {
            return 'token';
        }
        if (charm.app_id.startsWith('B/')) {
            return 'dapp';
        }
        // Any other prefix is considered 'other'
        return 'other';
    }

    // Fallback: check apps in data structure
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
                if (appValue.startsWith('B/')) {
                    return 'dapp';
                }
            }
        }
    }

    return 'unknown';
};


// Transforms a charm object from the API to the format expected by the UI
export const transformCharmData = (charm) => {
    if (!charm) {
        return null;
    }

    const detectedType = detectCharmType(charm);

    const charmData = getNestedProperty(charm, 'data.data.outs[0].charms.$0000') || {};

    const hasApiData = getNestedProperty(charm, 'data.has_api_data');
    const noteOnly = getNestedProperty(charm, 'data.data.note') === "No charm data from API";

    const name = charmData.name ||
        getNestedProperty(charm, 'data.data.name') ||
        `Charm ${charm.charmid?.substring(0, 8) || 'Unknown'}`;

    const description = charmData.description ||
        getNestedProperty(charm, 'data.data.description') ||
        (noteOnly ? 'Charm detected but no metadata available' : 'No description available');

    const image = charmData.image ||
        getNestedProperty(charm, 'data.data.image') ||
        '/images/logo.png';

    const ticker = charmData.ticker ||
        getNestedProperty(charm, 'data.data.ticker') || '';

    const supply = getNestedProperty(charm, 'data.data.supply') || 0;
    const remaining = charmData.remaining ||
        getNestedProperty(charm, 'data.data.remaining') || 0;

    const url = charmData.url ||
        getNestedProperty(charm, 'data.data.url') || '';

    const attributes = getNestedProperty(charm, 'data.data.attributes') || [];

    // Extract additional metadata from the new structure
    const imageHash = charmData.image_hash || '';

    // Extract UTXO ID from the inputs
    const utxoId = getNestedProperty(charm, 'data.data.ins[0].utxo_id') || '';

    // Extract app information
    const appData = getNestedProperty(charm, 'data.data.apps.$0000') || '';

    // Extract version
    const version = getNestedProperty(charm, 'data.data.version') || '';

    const likes = charm.likes_count || 0;
    const userLiked = charm.user_liked || false;
    const comments = 0;

    const result = {
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
        userLiked,
        comments,
        ticker,
        supply,
        remaining,
        url,
        attributes,
        rawCharmData: charmData
    };

    return result;
};

// Transforms an array of charms from the API
export const transformCharmsArray = (charms) => {
    if (!Array.isArray(charms)) {
        return [];
    }

    const seenIds = new Set();
    const result = [];

    charms.forEach((charm, index) => {
        const transformed = transformCharmData(charm);
        if (transformed) {
            if (!transformed.id) {
                transformed.id = `generated-${index}-${Date.now()}`;
            }

            if (seenIds.has(transformed.id)) {
                let counter = 1;
                let newId = `${transformed.id}-${counter}`;
                while (seenIds.has(newId)) {
                    counter++;
                    newId = `${transformed.id}-${counter}`;
                }
                transformed.id = newId;
            }

            seenIds.add(transformed.id);
            result.push(transformed);
        }
    });

    return result;
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
        image: '/images/logo.png',
        txid: '',
        outputIndex: 0,
        address: '',
        createdAt: new Date().toISOString(),
        likes: 0,
        userLiked: false,
        comments: 0,
        ticker: '',
        supply: 0,
        remaining: 0,
        url: '',
        attributes: [],
        // New fields from the metadata standards
        imageHash: '',
        utxoId: '',
        appData: '',
        version: '',
        rawCharmData: {}
    };
};
