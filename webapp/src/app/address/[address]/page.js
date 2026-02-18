'use client';

export const runtime = 'edge';

import { useState, useEffect } from 'react';
import { useParams } from 'next/navigation';
import Link from 'next/link';
import { fetchCharmsByAddress } from '../../../services/apiServices';
import { fetchReferenceNftByHash, extractHashFromAppId } from '../../../services/api/referenceNft';

export default function AddressPage() {
    const params = useParams();
    const address = params.address;
    const [charms, setCharms] = useState([]);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState(null);
    const [groupedAssets, setGroupedAssets] = useState({});
    const [nftImages, setNftImages] = useState({});

    useEffect(() => {
        const loadCharms = async () => {
            try {
                setIsLoading(true);
                setError(null);

                const data = await fetchCharmsByAddress(address);
                const charmsData = data.charms || [];
                setCharms(charmsData);

                // Group charms by app_id for summary
                const grouped = {};
                const hashesToFetch = new Set();
                
                charmsData.forEach(charm => {
                    const appId = charm.charmid;
                    if (!grouped[appId]) {
                        grouped[appId] = {
                            app_id: appId,
                            asset_type: charm.asset_type,
                            charms: [],
                            total_amount: 0,
                            block_height: charm.block_height,
                            created_at: charm.date_created,
                            network: charm.network || 'mainnet',
                        };
                    }
                    grouped[appId].charms.push(charm);
                    const amount = charm.amount || 0;
                    grouped[appId].total_amount += amount;
                    
                    // Collect hashes for image fetching
                    if (appId?.startsWith('t/') || appId?.startsWith('n/')) {
                        const hash = extractHashFromAppId(appId);
                        if (hash) hashesToFetch.add(hash);
                    }
                });

                setGroupedAssets(grouped);

                // Fetch images for all unique hashes
                const images = {};
                for (const hash of hashesToFetch) {
                    try {
                        const refNft = await fetchReferenceNftByHash(hash);
                        if (refNft?.image_url) {
                            images[hash] = refNft.image_url;
                        }
                    } catch (e) {
                        // Ignore image fetch errors
                    }
                }
                setNftImages(images);

            } catch (error) {
                setError(error.message);
            } finally {
                setIsLoading(false);
            }
        };

        if (address) {
            loadCharms();
        }
    }, [address]);

    // Format amount with decimals
    const formatAmount = (amount, decimals = 8) => {
        const value = amount / Math.pow(10, decimals);
        if (value >= 1000000) return (value / 1000000).toFixed(2) + 'M';
        if (value >= 1000) return (value / 1000).toFixed(2) + 'K';
        return value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    };

    // Get image for a charm
    const getCharmImage = (appId) => {
        const hash = extractHashFromAppId(appId);
        return hash ? nftImages[hash] : null;
    };

    // Get asset name from app_id
    const getAssetName = (appId) => {
        if (appId?.includes('3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b')) {
            return 'Bro';
        }
        return appId?.substring(0, 12) + '...';
    };

    return (
        <div className="min-h-screen bg-dark-900">
            <div className="bg-dark-900 pt-24 pb-6">
                <div className="container mx-auto px-4">
                    <h1 className="text-3xl font-bold mb-3 gradient-text">Address Portfolio</h1>
                    <div className="flex flex-col sm:flex-row sm:items-center gap-3 mb-4">
                        <p className="text-dark-300 font-mono text-sm break-all flex-1">
                            {address}
                        </p>
                        <a
                            href={`https://mempool.space/address/${address}`}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="inline-flex items-center gap-2 px-4 py-2 bg-orange-600/20 hover:bg-orange-600/40 border border-orange-500/50 rounded-lg text-orange-300 hover:text-orange-200 transition-colors text-sm font-medium whitespace-nowrap"
                        >
                            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
                                <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"/>
                            </svg>
                            View on Mempool.space ↗
                        </a>
                    </div>

                    {!isLoading && !error && (
                        <div className="flex items-center space-x-6 text-sm">
                            <div className="flex items-center space-x-2">
                                <span className="text-dark-400">Total UTXOs:</span>
                                <span className="text-primary-400 font-bold text-lg">{charms.length}</span>
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

            {isLoading && (
                <div className="container mx-auto px-4 py-16 flex justify-center">
                    <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-500"></div>
                </div>
            )}

            {!error && !isLoading && (
                <div className="container mx-auto px-4 py-6">
                    {charms.length === 0 ? (
                        <div className="py-16 text-center">
                            <h3 className="text-xl font-medium text-gray-300 mb-2">No unspent charms found</h3>
                            <p className="text-gray-400">This address has no unspent charm UTXOs</p>
                        </div>
                    ) : (
                        <>
                            {/* Summary by Asset - Primary View */}
                            {Object.keys(groupedAssets).length > 0 && (
                                <div className="mb-8">
                                    <h2 className="text-xl font-bold text-white mb-4">
                                        Holdings ({Object.keys(groupedAssets).length} asset{Object.keys(groupedAssets).length !== 1 ? 's' : ''})
                                    </h2>
                                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                        {Object.values(groupedAssets).map(group => {
                                            const image = getCharmImage(group.app_id);
                                            return (
                                                <div 
                                                    key={group.app_id}
                                                    className="bg-dark-800/50 rounded-lg p-4 border border-dark-700"
                                                >
                                                    <div className="flex items-center gap-3 mb-3">
                                                        {image ? (
                                                            <img src={image} alt="" className="w-12 h-12 rounded-lg object-cover" />
                                                        ) : (
                                                            <div className="w-12 h-12 rounded-lg bg-dark-700" />
                                                        )}
                                                        <div>
                                                            <Link 
                                                                href={`/asset/${encodeURIComponent(group.app_id)}`}
                                                                className="text-white font-medium hover:text-primary-400"
                                                            >
                                                                {getAssetName(group.app_id)}
                                                            </Link>
                                                            <div className="text-xs text-dark-400">{group.asset_type}</div>
                                                        </div>
                                                    </div>
                                                    <div className="grid grid-cols-2 gap-2 text-sm">
                                                        <div>
                                                            <div className="text-dark-400">Total Amount</div>
                                                            <div className="text-white font-semibold">{formatAmount(group.total_amount)}</div>
                                                        </div>
                                                        <div>
                                                            <div className="text-dark-400">UTXOs</div>
                                                            <div className="text-primary-400 font-semibold">{group.charms.length}</div>
                                                        </div>
                                                    </div>
                                                </div>
                                            );
                                        })}
                                    </div>
                                </div>
                            )}

                            {/* Individual UTXOs - Collapsed by default */}
                            <details className="group">
                                <summary className="text-lg font-bold text-white mb-4 cursor-pointer list-none flex items-center gap-2">
                                    <span className="text-dark-400 group-open:rotate-90 transition-transform">▶</span>
                                    Individual UTXOs ({charms.length})
                                </summary>
                                <div className="bg-dark-800/50 rounded-lg overflow-x-auto mt-4">
                                    <table className="w-full min-w-[800px]">
                                        <thead>
                                            <tr className="border-b border-dark-700">
                                                <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium">Asset</th>
                                                <th className="text-right px-4 py-3 text-dark-400 text-sm font-medium">Amount</th>
                                                <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium">UTXO</th>
                                                <th className="text-center px-4 py-3 text-dark-400 text-sm font-medium">Block</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {charms.map((charm, index) => {
                                                const amount = charm.amount || 0;
                                                const vout = charm.vout ?? 0;
                                                return (
                                                    <tr 
                                                        key={`${charm.txid}-${vout}-${index}`}
                                                        className="border-b border-dark-700/50 hover:bg-dark-700/30 transition-colors"
                                                    >
                                                        <td className="px-4 py-3">
                                                            <Link 
                                                                href={`/asset/${encodeURIComponent(charm.charmid)}`}
                                                                className="text-white hover:text-primary-400"
                                                            >
                                                                {getAssetName(charm.charmid)}
                                                            </Link>
                                                        </td>
                                                        <td className="px-4 py-3 text-right text-white">
                                                            {formatAmount(amount)}
                                                        </td>
                                                        <td className="px-4 py-3">
                                                            <Link 
                                                                href={`/tx?txid=${charm.txid}`}
                                                                className="text-primary-400 hover:text-primary-300 font-mono text-sm"
                                                            >
                                                                {charm.txid?.substring(0, 8)}...:{vout}
                                                            </Link>
                                                        </td>
                                                        <td className="px-4 py-3 text-center text-dark-300 text-sm">
                                                            #{charm.block_height?.toLocaleString()}
                                                        </td>
                                                    </tr>
                                                );
                                            })}
                                        </tbody>
                                    </table>
                                </div>
                            </details>
                        </>
                    )}
                </div>
            )}
        </div>
    );
}
