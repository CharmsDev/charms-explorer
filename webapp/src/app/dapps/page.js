'use client';

import { useState, useEffect } from 'react';
import FilterTabs from '../../components/FilterTabs';
import AssetGrid from '../../components/AssetGrid';
import { fetchAssets, getAssetCounts } from '../../services/api';

export default function DAppsPage() {
    const [assets, setAssets] = useState([]);
    const [counts, setCounts] = useState({ total: 0, nft: 0, token: 0, dapp: 0 });
    const [isLoading, setIsLoading] = useState(true);

    useEffect(() => {
        const loadData = async () => {
            try {
                setIsLoading(true);

                // Fetch asset counts
                const countsData = await getAssetCounts();
                setCounts(countsData);

                // Fetch dApp assets
                const response = await fetchAssets('dapp');
                setAssets(response.data);
            } catch (error) {
                console.error('Error loading data:', error);
            } finally {
                setIsLoading(false);
            }
        };

        loadData();
    }, []);

    return (
        <div>
            <div className="bg-dark-900 pt-8 pb-6">
                <div className="container mx-auto px-4 text-center">
                    <h1 className="text-4xl font-bold mt-4 mb-3 gradient-text">Charms dApps</h1>
                    <p className="text-xl max-w-2xl mx-auto mb-2 text-dark-200">
                        Decentralized applications built on Bitcoin with Charms technology
                    </p>
                </div>
            </div>

            <FilterTabs counts={counts} />

            {/* Validation Box */}
            <div className="container mx-auto px-4 py-8">
                <div className="max-w-2xl mx-auto bg-gradient-to-br from-purple-900/30 to-indigo-900/30 rounded-lg p-8 text-center shadow-lg border border-purple-500/20">
                    <h3 className="text-2xl font-bold mb-4 text-purple-300">Submit and validate your Charms DApp</h3>
                    <p className="text-gray-300 mb-6">
                        Join the growing ecosystem of decentralized applications built on Bitcoin with Charms technology.
                        Submit your DApp for validation and showcase it to the community.
                    </p>
                    <button
                        className="px-8 py-3 bg-gray-600 text-white rounded-md font-medium opacity-60 cursor-not-allowed"
                        disabled
                    >
                        Start
                    </button>
                    <p className="text-gray-500 text-sm mt-3">Validation process is not yet available</p>
                </div>
            </div>

            <div className="container mx-auto px-4 py-6">
                <div className="flex justify-between items-center mb-6">
                    <h2 className="text-2xl font-bold">
                        Found <span className="text-primary-500">{counts.dapp.toLocaleString()}</span> dApps
                    </h2>
                    <div className="flex space-x-2">
                        <select className="bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-md px-3 py-2 text-sm">
                            <option>Newest First</option>
                            <option>Oldest First</option>
                        </select>
                    </div>
                </div>
            </div>

            <AssetGrid assets={assets} isLoading={isLoading} />
        </div>
    );
}
