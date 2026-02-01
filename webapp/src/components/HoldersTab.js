// [RJJ-STATS-HOLDERS] Component to display asset holder statistics - Dark Mode

'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { fetchAssetHolders } from '@/services/apiServices';

export default function HoldersTab({ appId }) {
    const [holders, setHolders] = useState([]);
    const [totalHolders, setTotalHolders] = useState(0);
    const [totalSupply, setTotalSupply] = useState(0);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);

    useEffect(() => {
        const loadHolders = async () => {
            if (!appId) return;

            try {
                setLoading(true);
                setError(null);
                
                const data = await fetchAssetHolders(appId);
                
                setHolders(data.holders || []);
                setTotalHolders(data.total_holders || 0);
                setTotalSupply(data.total_supply || 0);
            } catch (err) {
                console.error('Error loading holders:', err);
                setError('Failed to load holder data');
            } finally {
                setLoading(false);
            }
        };

        loadHolders();
    }, [appId]);

    // Format supply with proper decimals
    const formatAmount = (amount) => {
        const value = amount / 100000000;
        if (value >= 1000000) return (value / 1000000).toFixed(2) + 'M';
        if (value >= 1000) return (value / 1000).toFixed(2) + 'K';
        return value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    };

    if (loading) {
        return (
            <div className="flex justify-center items-center py-12">
                <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-500"></div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="bg-red-500/10 border border-red-500/20 rounded-lg p-4 text-red-400">
                {error}
            </div>
        );
    }

    if (totalHolders === 0) {
        return (
            <div className="bg-dark-800/50 border border-dark-700 rounded-lg p-8 text-center text-dark-400">
                No holders found for this asset
            </div>
        );
    }

    return (
        <div className="space-y-6">
            {/* Summary Stats - Dark Mode */}
            <div className="flex items-center justify-between">
                <div className="text-dark-400">
                    <span className="text-primary-400 font-semibold">{totalHolders.toLocaleString()}</span> holders
                </div>
                <div className="text-dark-400">
                    Total Supply: <span className="text-white font-semibold">{formatAmount(totalSupply)}</span>
                </div>
            </div>

            {/* Holders Table - Dark Mode like Transactions */}
            <div className="bg-dark-800/50 rounded-lg overflow-x-auto">
                <table className="w-full min-w-[700px]">
                    <thead>
                        <tr className="border-b border-dark-700">
                            <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium whitespace-nowrap">Rank</th>
                            <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium whitespace-nowrap">Address</th>
                            <th className="text-right px-4 py-3 text-dark-400 text-sm font-medium whitespace-nowrap">Amount</th>
                            <th className="text-right px-4 py-3 text-dark-400 text-sm font-medium whitespace-nowrap">% Supply</th>
                            <th className="text-right px-4 py-3 text-dark-400 text-sm font-medium whitespace-nowrap">Charms</th>
                        </tr>
                    </thead>
                    <tbody>
                        {holders.map((holder, index) => (
                            <tr 
                                key={holder.address}
                                className="border-b border-dark-700/50 hover:bg-dark-700/30 transition-colors"
                            >
                                <td className="px-4 py-3 text-dark-300 text-sm">
                                    #{index + 1}
                                </td>
                                <td className="px-4 py-3">
                                    <Link 
                                        href={`/address/${holder.address}`}
                                        className="text-primary-400 hover:text-primary-300 font-mono text-sm"
                                    >
                                        {holder.address.slice(0, 12)}...{holder.address.slice(-8)}
                                    </Link>
                                </td>
                                <td className="px-4 py-3 text-right text-white text-sm font-medium">
                                    {formatAmount(holder.total_amount)}
                                </td>
                                <td className="px-4 py-3 text-right">
                                    <span className={`px-2 py-1 rounded text-xs font-medium whitespace-nowrap ${
                                        holder.percentage >= 10 ? 'bg-red-500/20 text-red-400' :
                                        holder.percentage >= 5 ? 'bg-orange-500/20 text-orange-400' :
                                        holder.percentage >= 1 ? 'bg-yellow-500/20 text-yellow-400' :
                                        'bg-green-500/20 text-green-400'
                                    }`}>
                                        {holder.percentage.toFixed(2)}%
                                    </span>
                                </td>
                                <td className="px-4 py-3 text-right text-dark-300 text-sm">
                                    {holder.charm_count.toLocaleString()}
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            </div>

            {/* Distribution Info - Dark Mode */}
            {holders.length >= 10 && (
                <div className="text-sm text-dark-400">
                    Top 10 holders control{' '}
                    <span className="text-primary-400 font-semibold">
                        {holders.slice(0, 10).reduce((sum, h) => sum + h.percentage, 0).toFixed(2)}%
                    </span>{' '}
                    of the total supply
                </div>
            )}
        </div>
    );
}
