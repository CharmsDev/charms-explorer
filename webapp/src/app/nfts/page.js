'use client';

import { useState, useEffect } from 'react';
import FilterTabs from '../../components/FilterTabs';
import AssetGrid from '../../components/AssetGrid';
import { fetchAssets, getAssetCounts } from '../../services/api';

export default function NFTsPage() {
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

                // Fetch NFT assets (page 1, limit 100 to get all)
                const response = await fetchAssets('nft', 1, 100);
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
            <div className="bg-purple-700 text-white py-12">
                <div className="container mx-auto px-4 text-center">
                    <h1 className="text-4xl font-bold mb-4">Charms NFTs</h1>
                    <p className="text-xl max-w-2xl mx-auto">
                        Unique digital collectibles on Bitcoin powered by Charms
                    </p>
                </div>
            </div>

            <FilterTabs counts={counts} />

            <div className="container mx-auto px-4 py-6">
                <div className="flex justify-between items-center mb-6">
                    <h2 className="text-2xl font-bold">
                        Found <span className="text-purple-600">{counts.nft.toLocaleString()}</span> NFTs
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
