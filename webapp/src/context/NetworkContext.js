'use client';

import { createContext, useState, useContext, useCallback, useRef, useEffect } from 'react';

const NetworkContext = createContext();

const STORAGE_KEY = 'charms-explorer-networks';

// Default network state
const DEFAULT_NETWORKS = {
    bitcoinMainnet: true,
    bitcoinTestnet4: true,
    cardanoMainnet: false,
    cardanoPreprod: false
};

// Helper to compute network param from state
const computeNetworkParam = (networks) => {
    const bitcoinMainnetActive = networks.bitcoinMainnet;
    const bitcoinTestnet4Active = networks.bitcoinTestnet4;
    
    if (bitcoinMainnetActive && bitcoinTestnet4Active) {
        return 'all';
    } else if (bitcoinMainnetActive) {
        return 'mainnet';
    } else if (bitcoinTestnet4Active) {
        return 'testnet4';
    }
    return 'all';
};

// Load from localStorage (client-side only)
const loadFromStorage = () => {
    if (typeof window === 'undefined') return DEFAULT_NETWORKS;
    try {
        const stored = localStorage.getItem(STORAGE_KEY);
        if (stored) {
            const parsed = JSON.parse(stored);
            // Merge with defaults to handle new keys
            return { ...DEFAULT_NETWORKS, ...parsed };
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

export function NetworkProvider({ children, onNetworkChange }) {
    const [selectedNetworks, setSelectedNetworks] = useState(DEFAULT_NETWORKS);
    const [isHydrated, setIsHydrated] = useState(false);

    // Hydrate from localStorage on mount (client-side only)
    useEffect(() => {
        const stored = loadFromStorage();
        setSelectedNetworks(stored);
        setIsHydrated(true);
    }, []);

    // Store callback in ref to avoid dependency issues
    const onNetworkChangeRef = useRef(onNetworkChange);
    useEffect(() => {
        onNetworkChangeRef.current = onNetworkChange;
    }, [onNetworkChange]);

    const toggleNetwork = useCallback((network) => {
        // Ignore Cardano toggles (disabled)
        if (network === 'cardanoMainnet' || network === 'cardanoPreprod') {
            return;
        }
        
        setSelectedNetworks(prev => {
            const newNetworks = {
                ...prev,
                [network]: !prev[network]
            };
            
            // Save to localStorage
            saveToStorage(newNetworks);
            
            // Notify callback if Bitcoin networks changed
            if ((network === 'bitcoinMainnet' || network === 'bitcoinTestnet4') && onNetworkChangeRef.current) {
                const networkParam = computeNetworkParam(newNetworks);
                onNetworkChangeRef.current(networkParam);
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
