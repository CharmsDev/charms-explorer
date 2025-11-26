// [RJJ-STATS-HOLDERS] Component to display asset holder statistics

'use client';

import { useEffect, useState } from 'react';
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

    if (loading) {
        return (
            <div className="flex justify-center items-center py-12">
                <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-orange-500"></div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700">
                {error}
            </div>
        );
    }

    if (totalHolders === 0) {
        return (
            <div className="bg-gray-50 border border-gray-200 rounded-lg p-8 text-center text-gray-600">
                No holders found for this asset
            </div>
        );
    }

    return (
        <div className="space-y-6">
            {/* Summary Stats */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div className="bg-gradient-to-br from-orange-50 to-orange-100 border border-orange-200 rounded-lg p-6">
                    <div className="text-sm text-orange-600 font-medium mb-1">Total Holders</div>
                    <div className="text-3xl font-bold text-orange-900">{totalHolders.toLocaleString()}</div>
                </div>
                <div className="bg-gradient-to-br from-blue-50 to-blue-100 border border-blue-200 rounded-lg p-6">
                    <div className="text-sm text-blue-600 font-medium mb-1">Total Supply (UNSPENT)</div>
                    <div className="text-3xl font-bold text-blue-900">
                        {(totalSupply / 100000000).toLocaleString(undefined, { 
                            minimumFractionDigits: 2,
                            maximumFractionDigits: 8 
                        })}
                    </div>
                </div>
            </div>

            {/* Holders Table */}
            <div className="bg-white border border-gray-200 rounded-lg overflow-hidden">
                <div className="overflow-x-auto">
                    <table className="min-w-full divide-y divide-gray-200">
                        <thead className="bg-gray-50">
                            <tr>
                                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                    Rank
                                </th>
                                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                                    Address
                                </th>
                                <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                                    Amount
                                </th>
                                <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                                    % of Supply
                                </th>
                                <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                                    Charms
                                </th>
                            </tr>
                        </thead>
                        <tbody className="bg-white divide-y divide-gray-200">
                            {holders.map((holder, index) => (
                                <tr key={holder.address} className="hover:bg-gray-50 transition-colors">
                                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                        #{index + 1}
                                    </td>
                                    <td className="px-6 py-4 whitespace-nowrap">
                                        <a 
                                            href={`/address/${holder.address}`}
                                            className="text-sm font-mono text-blue-600 hover:text-blue-800 hover:underline"
                                        >
                                            {holder.address.slice(0, 12)}...{holder.address.slice(-8)}
                                        </a>
                                    </td>
                                    <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium text-gray-900">
                                        {(holder.total_amount / 100000000).toLocaleString(undefined, {
                                            minimumFractionDigits: 2,
                                            maximumFractionDigits: 8
                                        })}
                                    </td>
                                    <td className="px-6 py-4 whitespace-nowrap text-right text-sm text-gray-600">
                                        <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${
                                            holder.percentage >= 10 ? 'bg-red-100 text-red-800' :
                                            holder.percentage >= 5 ? 'bg-orange-100 text-orange-800' :
                                            holder.percentage >= 1 ? 'bg-yellow-100 text-yellow-800' :
                                            'bg-green-100 text-green-800'
                                        }`}>
                                            {holder.percentage.toFixed(2)}%
                                        </span>
                                    </td>
                                    <td className="px-6 py-4 whitespace-nowrap text-right text-sm text-gray-600">
                                        {holder.charm_count.toLocaleString()}
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            </div>

            {/* Distribution Info */}
            {holders.length > 0 && (
                <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
                    <div className="text-sm text-blue-800">
                        <strong>Top 10 holders</strong> control{' '}
                        <strong>
                            {holders.slice(0, 10).reduce((sum, h) => sum + h.percentage, 0).toFixed(2)}%
                        </strong>{' '}
                        of the total supply
                    </div>
                </div>
            )}
        </div>
    );
}
