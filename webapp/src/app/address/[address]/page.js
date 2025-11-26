'use client';

export const runtime = 'edge';

import { useState, useEffect } from 'react';
import { useParams } from 'next/navigation';
import AssetGrid from '../../../components/AssetGrid';
import { Button } from '../../../components/ui/Button';
import { fetchCharmsByAddress } from '../../../services/apiServices';

export default function AddressPage() {
    const params = useParams();
    const address = params.address;
    const [charms, setCharms] = useState([]);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState(null);
    const [groupedAssets, setGroupedAssets] = useState({});

    useEffect(() => {
        const loadCharms = async () => {
            try {
                setIsLoading(true);
                setError(null);

                const data = await fetchCharmsByAddress(address);

                setCharms(data.charms || []);

                // [RJJ-ADDRESS-SEARCH] Group charms by app_id and calculate totals
                const grouped = {};
                (data.charms || []).forEach(charm => {
                    const appId = charm.charmid; // charmid is actually app_id
                    if (!grouped[appId]) {
                        grouped[appId] = {
                            app_id: appId,
                            asset_type: charm.asset_type,
                            charms: [],
                            total_amount: 0,
                            block_height: charm.block_height,
                            created_at: charm.date_created,
                            network: 'mainnet', // Default
                        };
                    }
                    grouped[appId].charms.push(charm);
                    // Parse amount from charm data
                    const amount = charm.data?.amount || 0;
                    grouped[appId].total_amount += amount;
                });

                setGroupedAssets(grouped);

            } catch (error) {
                console.error('[AddressPage] Error loading charms:', error);
                setError(error.message);
            } finally {
                setIsLoading(false);
            }
        };

        if (address) {
            loadCharms();
        }
    }, [address]);

    // Transform grouped assets to display format
    const assetsForDisplay = Object.values(groupedAssets).map(group => ({
        id: group.app_id,
        app_id: group.app_id,
        asset_type: group.asset_type,
        name: group.app_id.substring(0, 20) + '...',
        symbol: group.asset_type.toUpperCase(),
        total_supply: group.total_amount,
        decimals: 8, // Default
        block_height: group.block_height,
        created_at: group.created_at,
        network: group.network,
        charm_count: group.charms.length,
    }));

    return (
        <div className="min-h-screen bg-dark-900">
            <div className="bg-dark-900 pt-24 pb-6">
                <div className="container mx-auto px-4">
                    <h1 className="text-3xl font-bold mb-3 gradient-text">Address Charms</h1>
                    <p className="text-dark-300 font-mono text-sm break-all mb-4">
                        {address}
                    </p>

                    {!isLoading && !error && (
                        <div className="flex items-center space-x-6 text-sm">
                            <div className="flex items-center space-x-2">
                                <span className="text-dark-400">Total Charms:</span>
                                <span className="text-white font-bold text-lg">{charms.length}</span>
                            </div>
                            <div className="flex items-center space-x-2">
                                <span className="text-dark-400">Unique Assets:</span>
                                <span className="text-white font-bold text-lg">{Object.keys(groupedAssets).length}</span>
                            </div>
                        </div>
                    )}
                </div>
            </div>

            {error && (
                <div className="container mx-auto px-4 py-8">
                    <div className="bg-red-900/20 border border-red-500/30 rounded-lg p-4 text-red-300">
                        <p className="font-medium">Error loading charms:</p>
                        <p className="text-sm mt-1">{error}</p>
                    </div>
                </div>
            )}

            {!error && (
                <>
                    <div className="container mx-auto px-4 py-4">
                        <div className="flex justify-between items-center">
                            <h2 className="text-xl font-bold text-white">
                                Assets Breakdown
                            </h2>
                        </div>
                    </div>

                    <AssetGrid assets={assetsForDisplay} isLoading={isLoading} />

                    {!isLoading && charms.length === 0 && (
                        <div className="container mx-auto px-4 py-16 text-center">
                            <h3 className="text-xl font-medium text-gray-300 mb-2">No unspent charms found</h3>
                            <p className="text-gray-400">This address has no unspent charm UTXOs</p>
                        </div>
                    )}
                </>
            )}
        </div>
    );
}
