'use client';

/**
 * Spell Metadata Parser
 * Normalizes spell data from different formats into a standard interface
 * Extracts ALL fields from spell data for display in asset details
 */

// Known standard fields that have special handling
const STANDARD_FIELDS = ['name', 'description', 'image', 'ticker', 'symbol', 'url', 'supply_limit', 'decimals'];

/**
 * Standard metadata interface returned by the parser
 * @typedef {Object} NormalizedMetadata
 * @property {string|null} name - Asset name
 * @property {string|null} description - Asset description
 * @property {string|null} image - Image URL or base64 data
 * @property {string|null} ticker - Token ticker/symbol
 * @property {string|null} url - External URL
 * @property {number|null} supply_limit - Max supply for tokens
 * @property {number|null} decimals - Token decimals
 * @property {Object} extraFields - All other dynamic fields from spell
 * @property {Object|null} raw - Raw extracted data for debugging
 */

/**
 * Extract all fields from spell output data
 * Separates standard fields from extra/custom fields
 * @param {Object} data - The output data object from spell
 * @returns {{ standard: Object, extra: Object }}
 */
function extractAllFields(data) {
    if (!data || typeof data !== 'object') {
        return { standard: {}, extra: {} };
    }

    const standard = {};
    const extra = {};

    for (const [key, value] of Object.entries(data)) {
        if (STANDARD_FIELDS.includes(key)) {
            standard[key] = value;
        } else {
            // All non-standard fields go to extra
            extra[key] = value;
        }
    }

    return { standard, extra };
}

/**
 * Parse spell native_data to extract metadata
 * Handles the structure: data.native_data.tx.outs[n]["0"] or data.native_data.tx.outs[n][0]
 * @param {Object} charm - The charm/asset object from API
 * @returns {NormalizedMetadata}
 */
export function parseSpellMetadata(charm) {
    const defaultMetadata = {
        name: null,
        description: null,
        image: null,
        ticker: null,
        url: null,
        supply_limit: null,
        decimals: null,
        extraFields: {},
        raw: null
    };

    if (!charm?.data) return defaultMetadata;

    // Try native_data structure (most common for spells)
    const nativeData = charm.data.native_data;
    if (nativeData?.tx?.outs) {
        for (const out of nativeData.tx.outs) {
            // Try both "0" (string key) and 0 (number key)
            const data = out["0"] || out[0];
            if (data && typeof data === 'object') {
                // Skip if it's just a number (amount)
                if (typeof data === 'number') continue;
                
                const { standard, extra } = extractAllFields(data);
                
                return {
                    name: standard.name || null,
                    description: standard.description || null,
                    image: standard.image || null,
                    ticker: standard.ticker || standard.symbol || null,
                    url: standard.url || null,
                    supply_limit: standard.supply_limit || null,
                    decimals: standard.decimals ?? null,
                    extraFields: extra,
                    raw: data
                };
            }
        }
    }

    // Try legacy spell_data structure
    const spellData = charm.data.spell_data;
    if (spellData?.outputs) {
        for (const output of spellData.outputs) {
            if (output?.metadata) {
                const { standard, extra } = extractAllFields(output.metadata);
                
                return {
                    name: standard.name || null,
                    description: standard.description || null,
                    image: standard.image || null,
                    ticker: standard.ticker || standard.symbol || null,
                    url: standard.url || null,
                    supply_limit: standard.supply_limit || null,
                    decimals: standard.decimals ?? null,
                    extraFields: extra,
                    raw: output.metadata
                };
            }
        }
    }

    // Try nested data.data structure (common API response format)
    if (charm.data.data && typeof charm.data.data === 'object') {
        const nestedData = charm.data.data;
        const { standard, extra } = extractAllFields(nestedData);
        
        return {
            name: standard.name || null,
            description: standard.description || null,
            image: standard.image || null,
            ticker: standard.ticker || standard.symbol || null,
            url: standard.url || null,
            supply_limit: standard.supply_limit || null,
            decimals: standard.decimals ?? null,
            extraFields: extra,
            raw: nestedData
        };
    }

    // Try direct data structure
    if (charm.data.name || charm.data.image) {
        const { standard, extra } = extractAllFields(charm.data);
        
        return {
            name: standard.name || null,
            description: standard.description || null,
            image: standard.image || null,
            ticker: standard.ticker || standard.symbol || null,
            url: standard.url || null,
            supply_limit: standard.supply_limit || null,
            decimals: standard.decimals ?? null,
            extraFields: extra,
            raw: charm.data
        };
    }

    return defaultMetadata;
}

/**
 * Get display-ready metadata for a charm
 * Combines spell metadata with other sources (NFT reference, asset fields)
 * @param {Object} charm - The charm/asset object
 * @param {Object} nftReference - Optional NFT reference metadata for tokens
 * @returns {NormalizedMetadata}
 */
export function getDisplayMetadata(charm, nftReference = null) {
    const spellMeta = parseSpellMetadata(charm);
    
    return {
        name: spellMeta.name || nftReference?.name || charm?.name || null,
        description: spellMeta.description || nftReference?.description || charm?.description || null,
        image: spellMeta.image || nftReference?.image_url || charm?.image_url || null,
        ticker: spellMeta.ticker || nftReference?.symbol || charm?.symbol || null,
        url: spellMeta.url || nftReference?.url || charm?.url || null,
        supply_limit: spellMeta.supply_limit || charm?.supply_limit || null,
        decimals: spellMeta.decimals ?? charm?.decimals ?? null,
        extraFields: spellMeta.extraFields || {},
        raw: spellMeta.raw
    };
}

/**
 * Check if metadata has a valid image
 * @param {NormalizedMetadata} metadata
 * @returns {boolean}
 */
export function hasValidImage(metadata) {
    return !!(metadata?.image && metadata.image.length > 0);
}

/**
 * Check if image is base64 encoded
 * @param {string} image
 * @returns {boolean}
 */
export function isBase64Image(image) {
    return image?.startsWith('data:image/') || false;
}

/**
 * Check if image is a URL (http/https)
 * @param {string} image
 * @returns {boolean}
 */
export function isImageUrl(image) {
    return image?.startsWith('http://') || image?.startsWith('https://') || false;
}

/**
 * Get the appropriate image source for display
 * Handles both base64 and URL images
 * @param {string} image - Image string (base64 or URL)
 * @returns {string|null} - Valid image source or null
 */
export function getImageSource(image) {
    if (!image) return null;
    if (isBase64Image(image)) return image;
    if (isImageUrl(image)) return image;
    return null;
}

/**
 * Format a field name for display (snake_case to Title Case)
 * @param {string} fieldName
 * @returns {string}
 */
export function formatFieldName(fieldName) {
    if (!fieldName) return '';
    return fieldName
        .split('_')
        .map(word => word.charAt(0).toUpperCase() + word.slice(1))
        .join(' ');
}

/**
 * Check if a value is a byte array (array of numbers 0-255)
 * @param {any} value
 * @returns {boolean}
 */
export function isByteArray(value) {
    if (!Array.isArray(value)) return false;
    if (value.length === 0) return false;
    // Check if all elements are integers between 0-255
    return value.every(v => 
        typeof v === 'number' && 
        Number.isInteger(v) && 
        v >= 0 && 
        v <= 255
    );
}

/**
 * Convert a byte array to hexadecimal string
 * @param {number[]} bytes - Array of bytes (0-255)
 * @returns {string} - Hex string with 0x prefix
 */
export function byteArrayToHex(bytes) {
    if (!Array.isArray(bytes)) return '';
    const hex = bytes.map(b => b.toString(16).padStart(2, '0')).join('');
    return `0x${hex}`;
}

/**
 * Truncate a hex string for display, showing start and end
 * @param {string} hex - Hex string
 * @param {number} showChars - Number of chars to show at start/end (default 8)
 * @returns {string}
 */
export function truncateHex(hex, showChars = 8) {
    if (!hex || hex.length <= showChars * 2 + 4) return hex;
    const prefix = hex.startsWith('0x') ? '0x' : '';
    const cleanHex = hex.startsWith('0x') ? hex.slice(2) : hex;
    if (cleanHex.length <= showChars * 2) return hex;
    return `${prefix}${cleanHex.slice(0, showChars)}...${cleanHex.slice(-showChars)}`;
}

/**
 * Format a complex value (object/array) for display
 * Detects byte arrays and converts to hex
 * @param {any} value
 * @returns {{ formatted: string, isHex: boolean, fullValue: string|null }}
 */
export function formatComplexValue(value) {
    // Check for byte array first
    if (isByteArray(value)) {
        const fullHex = byteArrayToHex(value);
        return {
            formatted: truncateHex(fullHex, 8),
            isHex: true,
            fullValue: fullHex
        };
    }
    
    // Handle nested objects that might contain byte arrays
    if (typeof value === 'object' && value !== null) {
        const processed = processObjectForDisplay(value);
        const jsonStr = JSON.stringify(processed, null, 0);
        const truncated = jsonStr.length > 100 
            ? jsonStr.substring(0, 100) + '...' 
            : jsonStr;
        return {
            formatted: truncated,
            isHex: false,
            fullValue: JSON.stringify(processed, null, 2)
        };
    }
    
    return {
        formatted: String(value),
        isHex: false,
        fullValue: null
    };
}

/**
 * Process an object recursively, converting byte arrays to hex
 * @param {any} obj
 * @returns {any}
 */
export function processObjectForDisplay(obj) {
    if (obj === null || obj === undefined) return obj;
    
    if (isByteArray(obj)) {
        return byteArrayToHex(obj);
    }
    
    if (Array.isArray(obj)) {
        return obj.map(item => processObjectForDisplay(item));
    }
    
    if (typeof obj === 'object') {
        const result = {};
        for (const [key, value] of Object.entries(obj)) {
            result[key] = processObjectForDisplay(value);
        }
        return result;
    }
    
    return obj;
}

/**
 * Format a field value for display
 * Handles different types: strings, numbers, booleans, objects, byte arrays
 * @param {any} value
 * @returns {string}
 */
export function formatFieldValue(value) {
    if (value === null || value === undefined) return '-';
    if (typeof value === 'boolean') return value ? 'Yes' : 'No';
    if (typeof value === 'number') return value.toLocaleString();
    if (typeof value === 'string') {
        // Truncate very long strings (like hashes)
        if (value.length > 66) {
            return `${value.substring(0, 10)}...${value.substring(value.length - 10)}`;
        }
        return value;
    }
    // Handle arrays and objects
    if (typeof value === 'object') {
        const { formatted } = formatComplexValue(value);
        return formatted;
    }
    return String(value);
}
