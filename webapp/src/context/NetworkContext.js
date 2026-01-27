'use client';

import { createContext, useState, useContext, useCallback, useRef, useEffect } from 'react';

const NetworkContext = createContext();

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

export function NetworkProvider({ children, onNetworkChange }) {
    const [selectedBlockchains, setSelectedBlockchains] = useState({
        bitcoin: true,
        cardano: true
    });

    const [selectedNetworks, setSelectedNetworks] = useState({
        bitcoinMainnet: true,
        bitcoinTestnet4: true,
        cardanoMainnet: false,
        cardanoPreprod: false
    });

    // Store callback in ref to avoid dependency issues
    const onNetworkChangeRef = useRef(onNetworkChange);
    useEffect(() => {
        onNetworkChangeRef.current = onNetworkChange;
    }, [onNetworkChange]);

    const toggleNetwork = useCallback((network) => {
        setSelectedNetworks(prev => {
            const newNetworks = {
                ...prev,
                [network]: !prev[network]
            };
            
            // Notify callback if Bitcoin networks changed
            if ((network === 'bitcoinMainnet' || network === 'bitcoinTestnet4') && onNetworkChangeRef.current) {
                const networkParam = computeNetworkParam(newNetworks);
                onNetworkChangeRef.current(networkParam);
            }
            
            return newNetworks;
        });
    }, []);

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
            shouldDisplayAsset
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
