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
    if (!charm) {
        console.log('transformCharmData: charm is null/undefined');
        return null;
    }

    console.log('Transforming charm:', charm.charmid, 'data:', charm.data);
    
    const detectedType = detectCharmType(charm);
    console.log('Detected type for charm', charm.charmid, ':', detectedType);

    // Extract data from the new metadata structure
    // First check for data in the standard metadata structure (outs[0].charms.$0000)
    const charmData = getNestedProperty(charm, 'data.data.outs[0].charms.$0000') || {};

    // Check if this charm has actual data or just a "No charm data from API" note
    const hasApiData = getNestedProperty(charm, 'data.has_api_data');
    const noteOnly = getNestedProperty(charm, 'data.data.note') === "No charm data from API";
    
    console.log('Charm has API data:', hasApiData, 'Note only:', noteOnly);
    
    // If not found in the new structure, fall back to the old structure
    const name = charmData.name ||
        getNestedProperty(charm, 'data.data.name') ||
        `Charm ${charm.charmid?.substring(0, 8) || 'Unknown'}`;

    const description = charmData.description ||
        getNestedProperty(charm, 'data.data.description') ||
        (noteOnly ? 'Charm detected but no metadata available' : 'No description available');

    const image = charmData.image ||
        getNestedProperty(charm, 'data.data.image') ||
        'https://charms.dev/_astro/logo-charms-dark.Ceshk2t3.png';

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

    // Use real likes count from API if available, otherwise default to 0
    const likes = charm.likes_count || 0;
    const userLiked = charm.user_liked || false;
    // Comments are not yet implemented, so we'll show 0
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
        // New fields from the metadata standards
        imageHash,
        utxoId,
        appData,
        version,
        // Store the raw charm data for debugging and future use
        rawCharmData: charmData
    };
    
    console.log('Transformed charm result:', result);
    return result;
};

// Transforms an array of charms from the API
export const transformCharmsArray = (charms) => {
    if (!Array.isArray(charms)) {
        console.log('transformCharmsArray: input is not an array:', charms);
        return [];
    }
    console.log('transformCharmsArray: processing', charms.length, 'charms');
    const transformed = charms.map(transformCharmData).filter(Boolean);
    console.log('transformCharmsArray: returned', transformed.length, 'transformed charms');
    return transformed;
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
