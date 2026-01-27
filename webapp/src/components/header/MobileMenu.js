'use client';

import Link from 'next/link';
import { useNetwork } from '@/context/NetworkContext';

export default function MobileMenu({ isOpen, onClose }) {
    const { selectedNetworks, toggleNetwork } = useNetwork();

    if (!isOpen) return null;

    return (
        <div className="md:hidden bg-dark-900/95 backdrop-blur-md border-t border-dark-800">
            <div className="container mx-auto px-4 py-3">
                <nav className="flex flex-col space-y-2">
                    {/* Mobile navigation links */}
                    <Link
                        href="/status"
                        className="flex items-center p-2 rounded-lg hover:bg-dark-800 transition-colors"
                        onClick={onClose}
                    >
                        <svg className="mr-2 h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"></path>
                        </svg>
                        Indexer Status
                    </Link>
                </nav>

                {/* Mobile network selection */}
                <div className="mt-4 border-t border-dark-800 pt-4">
                    <p className="text-xs text-dark-400 mb-2">Blockchain</p>
                    <div className="flex flex-wrap gap-2 mb-4">
                        <div className="px-3 py-1 text-xs text-white">
                            Bitcoin
                        </div>
                        <div className="px-3 py-1 text-xs text-white">
                            Cardano
                        </div>
                    </div>

                    <p className="text-xs text-dark-400 mb-2">Bitcoin Network</p>
                    <div className="flex flex-wrap gap-2 mb-4">
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

                    <p className="text-xs text-dark-400 mb-2">Cardano Network</p>
                    <div className="flex flex-wrap gap-2 mb-4">
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

                <div className="mt-4">
                    <div className="relative">
                        <input
                            type="text"
                            placeholder="Search Charms"
                            className="w-full bg-dark-800/70 text-white rounded-lg py-2 px-4 pl-10 focus:outline-none focus:ring-2 focus:ring-primary-500"
                        />
                        <div className="absolute left-3 top-2.5 text-dark-400">
                            <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                            </svg>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
