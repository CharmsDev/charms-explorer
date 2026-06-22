'use client';

import { createContext, useState, useContext, useCallback, useRef, useEffect } from 'react';

const NetworkContext = createContext();

const STORAGE_KEY = 'charms-explorer-networks';

// Default network state — single-select: only one Bitcoin network active at a time
const DEFAULT_NETWORKS = {
    bitcoinMainnet: true,
    bitcoinTestnet4: false,
    cardanoMainnet: false,
    cardanoPreprod: false
};

// Normalize ensures exactly one Bitcoin network is active (fixes old multi-select localStorage)
const normalizeNetworks = (networks) => {
    if (networks.bitcoinMainnet && networks.bitcoinTestnet4) {
        return { ...networks, bitcoinTestnet4: false };
    }
    if (!networks.bitcoinMainnet && !networks.bitcoinTestnet4) {
        return { ...networks, bitcoinMainnet: true };
    }
    return networks;
};

// Helper to compute network param from state
const computeNetworkParam = (networks) => {
    if (networks.bitcoinTestnet4) return 'testnet4';
    return 'mainnet';
};

// Load from localStorage (client-side only)
const loadFromStorage = () => {
    if (typeof window === 'undefined') return DEFAULT_NETWORKS;
    try {
        const stored = localStorage.getItem(STORAGE_KEY);
        if (stored) {
            const parsed = JSON.parse(stored);
            return normalizeNetworks({ ...DEFAULT_NETWORKS, ...parsed });
        }
    } catch (e) {
        console.warn('Failed to load network settings from localStorage:', e);
    }
    return DEFAULT_NETWORKS;
};

// Save to localStorage
const saveToStorage = (networks) => {
    if (typeof window === 'undefined') return;
    try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(networks));
    } catch (e) {
        console.warn('Failed to save network settings to localStorage:', e);
    }
};

// Convert a URL network param to network state
const networkParamToState = (param) => {
    if (param === 'testnet4') {
        return { ...DEFAULT_NETWORKS, bitcoinMainnet: false, bitcoinTestnet4: true };
    }
    if (param === 'mainnet') {
        return { ...DEFAULT_NETWORKS, bitcoinMainnet: true, bitcoinTestnet4: false };
    }
    return null; // invalid/missing — use stored/default
};

export function NetworkProvider({ children, onNetworkChange }) {
    const [selectedNetworks, setSelectedNetworks] = useState(DEFAULT_NETWORKS);
    const [isHydrated, setIsHydrated] = useState(false);

    // Hydrate: URL ?network= overrides localStorage
    useEffect(() => {
        const urlParams = new URLSearchParams(window.location.search);
        const urlNetwork = urlParams.get('network');
        const fromUrl = urlNetwork ? networkParamToState(urlNetwork) : null;
        if (fromUrl) {
            setSelectedNetworks(fromUrl);
            saveToStorage(fromUrl);
        } else {
            setSelectedNetworks(loadFromStorage());
        }
        setIsHydrated(true);
    }, []);

    // Store callback in ref to avoid dependency issues
    const onNetworkChangeRef = useRef(onNetworkChange);
    useEffect(() => {
        onNetworkChangeRef.current = onNetworkChange;
    }, [onNetworkChange]);

    const toggleNetwork = useCallback((network) => {
        if (network === 'cardanoMainnet' || network === 'cardanoPreprod') return;

        setSelectedNetworks(prev => {
            // Already selected — no-op (radio: can't deselect current)
            if (prev[network]) return prev;

            // Select this network, deselect all others in the Bitcoin group
            const newNetworks = {
                ...prev,
                bitcoinMainnet: false,
                bitcoinTestnet4: false,
                [network]: true,
            };

            saveToStorage(newNetworks);

            if (onNetworkChangeRef.current) {
                onNetworkChangeRef.current(computeNetworkParam(newNetworks));
            }

            return newNetworks;
        });
    }, []);

    // Get current network param for API calls
    const getNetworkParam = useCallback(() => {
        return computeNetworkParam(selectedNetworks);
    }, [selectedNetworks]);

    // Get active networks
    const getActiveNetworks = () => {
        const activeNetworks = [];

        if (selectedNetworks.bitcoinMainnet) {
            activeNetworks.push({ blockchain: 'bitcoin', network: 'mainnet' });
        }

        if (selectedNetworks.bitcoinTestnet4) {
            activeNetworks.push({ blockchain: 'bitcoin', network: 'testnet4' });
        }

        if (selectedNetworks.cardanoMainnet) {
            activeNetworks.push({ blockchain: 'cardano', network: 'mainnet' });
        }

        if (selectedNetworks.cardanoPreprod) {
            activeNetworks.push({ blockchain: 'cardano', network: 'preprod' });
        }

        return activeNetworks;
    };

    // Check if an asset should be displayed based on its blockchain and network
    const shouldDisplayAsset = (assetBlockchain, assetNetwork) => {
        if (assetBlockchain === 'bitcoin') {
            if (assetNetwork === 'mainnet') {
                return selectedNetworks.bitcoinMainnet;
            } else if (assetNetwork === 'testnet4') {
                return selectedNetworks.bitcoinTestnet4;
            }
        } else if (assetBlockchain === 'cardano') {
            if (assetNetwork === 'mainnet') {
                return selectedNetworks.cardanoMainnet;
            } else if (assetNetwork === 'preprod') {
                return selectedNetworks.cardanoPreprod;
            }
        }
        return false;
    };

    return (
        <NetworkContext.Provider value={{
            selectedNetworks,
            toggleNetwork,
            getActiveNetworks,
            shouldDisplayAsset,
            getNetworkParam,
            isHydrated
        }}>
            {children}
        </NetworkContext.Provider>
    );
}

// Custom hook to use the network context
export function useNetwork() {
    const context = useContext(NetworkContext);
    if (context === undefined) {
        throw new Error('useNetwork must be used within a NetworkProvider');
    }
    return context;
}
