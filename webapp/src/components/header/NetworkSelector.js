'use client';

import { useNetwork } from '@/context/NetworkContext';

export default function NetworkSelector() {
    const { selectedNetworks, toggleNetwork } = useNetwork();

    return (
        <div className="hidden md:flex justify-center items-center">
            {/* Bitcoin section */}
            <div className="flex items-center">
                <span className="text-sm font-medium text-white mr-2">Bitcoin</span>
                <div className="flex items-center space-x-1">
                    <button
                        onClick={() => toggleNetwork('bitcoinMainnet')}
                        className={`px-3 py-1 text-xs rounded-lg transition-colors ${selectedNetworks.bitcoinMainnet
                            ? 'bg-bitcoin-600 text-white'
                            : 'bg-dark-800 text-dark-400 hover:bg-dark-700'
                        }`}
                    >
                        Mainnet
                    </button>
                    <button
                        onClick={() => toggleNetwork('bitcoinTestnet4')}
                        className={`px-3 py-1 text-xs rounded-lg transition-colors ${selectedNetworks.bitcoinTestnet4
                            ? 'bg-bitcoin-600 text-white'
                            : 'bg-dark-800 text-dark-400 hover:bg-dark-700'
                        }`}
                    >
                        Testnet4
                    </button>
                </div>
            </div>

            <div className="mx-4"></div>

            {/* Cardano section */}
            <div className="flex items-center">
                <span className="text-sm font-medium text-white mr-2">Cardano</span>
                <div className="flex items-center space-x-1">
                    <button
                        onClick={() => toggleNetwork('cardanoMainnet')}
                        className={`px-3 py-1 text-xs rounded-lg transition-colors ${selectedNetworks.cardanoMainnet
                            ? 'bg-primary-600 text-white'
                            : 'bg-dark-800 text-dark-400 hover:bg-dark-700'
                        }`}
                    >
                        Mainnet
                    </button>
                    <button
                        onClick={() => toggleNetwork('cardanoPreprod')}
                        className={`px-3 py-1 text-xs rounded-lg transition-colors ${selectedNetworks.cardanoPreprod
                            ? 'bg-primary-600 text-white'
                            : 'bg-dark-800 text-dark-400 hover:bg-dark-700'
                        }`}
                    >
                        Preprod
                    </button>
                </div>
            </div>
        </div>
    );
}
