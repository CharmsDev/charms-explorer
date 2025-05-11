'use client';

import { createContext, useState, useContext } from 'react';

// Create the context
const NetworkContext = createContext();

// Create a provider component
export function NetworkProvider({ children }) {
    // Network selection states
    const [selectedBlockchains, setSelectedBlockchains] = useState({
        bitcoin: true,
        cardano: true
    });

    const [selectedNetworks, setSelectedNetworks] = useState({
        bitcoinMainnet: false,
        bitcoinTestnet4: true,
        cardanoMainnet: false,
        cardanoPreprod: false
    });

    // Toggle network selection
    const toggleNetwork = (network) => {
        setSelectedNetworks(prev => ({
            ...prev,
            [network]: !prev[network]
        }));
    };

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
