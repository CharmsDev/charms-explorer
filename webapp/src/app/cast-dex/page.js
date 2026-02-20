'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import Link from 'next/link';
import { fetchOpenDexOrders } from '@/services/api/dex';
import { useNetwork } from '@/context/NetworkContext';
import { useAutoRefresh } from '@/hooks/useAutoRefresh';

const CAST_APP_URL = process.env.NEXT_PUBLIC_CAST_APP_URL || 'https://cast.charms.dev';

// DEX tokens use 9 decimal places (10^9)
const TOKEN_DECIMALS = 9;

// Format token quantity from raw integer to display value
const formatTokenQuantity = (rawQuantity) => {
    if (!rawQuantity) return '-';
    const displayValue = rawQuantity / Math.pow(10, TOKEN_DECIMALS);
    // Format with up to 2 decimal places, removing trailing zeros
    return displayValue.toLocaleString(undefined, { 
        minimumFractionDigits: 0, 
        maximumFractionDigits: 2 
    });
};

// Get order type display info from API order data
const getOrderTypeDisplay = (order) => {
    const side = order.side?.toLowerCase();
    if (side === 'ask') return { type: 'Ask', color: 'bg-green-500/20 text-green-400', icon: '📈' };
    if (side === 'bid') return { type: 'Bid', color: 'bg-blue-500/20 text-blue-400', icon: '📉' };
    return { type: 'Order', color: 'bg-purple-500/20 text-purple-400', icon: '🔄' };
};

export default function CastDexPage() {
    const { getNetworkParam, isHydrated } = useNetwork();
    const [transactions, setTransactions] = useState([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);

    const loadCastTransactions = useCallback(async () => {
        try {
            setLoading(true);
            // Get current network filter
            const networkParam = getNetworkParam();
            
            // Fetch DEX orders directly from the dedicated API endpoint
            const result = await fetchOpenDexOrders(networkParam);
            const orders = result?.orders || [];
            
            setTransactions(orders);
            setError(null);
        } catch (err) {
            setError('Failed to load Cast Dex orders from indexer.');
        } finally {
            setLoading(false);
        }
    }, [getNetworkParam]);

    // Silent refresh: fetch latest orders and prepend any new ones
    const transactionsRef = useRef(transactions);
    transactionsRef.current = transactions;

    const silentRefresh = useCallback(async () => {
        const networkParam = getNetworkParam();
        const result = await fetchOpenDexOrders(networkParam);
        const freshOrders = result?.orders || [];
        if (freshOrders.length === 0) return;

        // Build a Set of existing order keys for fast lookup
        const existingKeys = new Set(
            transactionsRef.current.map(o => `${o.txid}:${o.vout}`)
        );
        const newOrders = freshOrders.filter(o => !existingKeys.has(`${o.txid}:${o.vout}`));

        if (newOrders.length > 0) {
            // Prepend new orders at the top
            setTransactions(prev => [...newOrders, ...prev]);
        }
    }, [getNetworkParam]);

    useAutoRefresh(silentRefresh, { enabled: isHydrated && !loading });

    useEffect(() => {
        if (isHydrated) {
            loadCastTransactions();
        }
    }, [isHydrated, loadCastTransactions]);

    const getMempoolUrl = (txid, network) => {
        if (!txid) return null;
        return network === 'mainnet'
            ? `https://mempool.space/tx/${txid}`
            : `https://mempool.space/testnet4/tx/${txid}`;
    };

    const formatDate = (dateStr) => {
        if (!dateStr) return '-';
        const date = new Date(dateStr);
        return date.toLocaleString();
    };

    return (
        <div className="min-h-screen">
            {/* Navigation tabs */}
            <div className="bg-dark-900/95 backdrop-blur-sm border-b border-dark-800 sticky top-16 z-40">
                <div className="container mx-auto px-4 py-3">
                    <div className="flex items-center justify-between gap-4">
                        <div className="flex items-center gap-2">
                            <Link 
                                href="/"
                                className="px-4 py-2 rounded-lg text-sm font-medium bg-dark-800 text-dark-300 hover:bg-dark-700 hover:text-white transition-all"
                            >
                                Charms
                            </Link>
                            <Link 
                                href="/transactions"
                                className="px-4 py-2 rounded-lg text-sm font-medium bg-dark-800 text-dark-300 hover:bg-dark-700 hover:text-white transition-all"
                            >
                                Transactions
                            </Link>
                            <Link 
                                href="/cast-dex"
                                className="px-4 py-2 rounded-lg text-sm font-medium bg-primary-600 text-white transition-all"
                            >
                                Cast Dex
                            </Link>
                        </div>
                        <div className="flex items-center gap-4">
                            <span className="text-dark-400">
                                <span className="text-primary-400 font-semibold">{transactions.length}</span> transactions
                            </span>
                            <a 
                                href={`${CAST_APP_URL}/orderbook`}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="px-3 py-1.5 bg-purple-600 hover:bg-purple-500 text-white rounded-lg text-sm font-medium transition-colors"
                            >
                                Order Book →
                            </a>
                        </div>
                    </div>
                </div>
            </div>

            {/* Transactions Table */}
            <div className="container mx-auto px-4 py-6">
                {loading ? (
                    <div className="flex justify-center items-center py-20">
                        <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-500"></div>
                    </div>
                ) : error ? (
                    <div className="text-center py-20">
                        <div className="text-red-400 mb-4">{error}</div>
                        <button 
                            onClick={loadCastTransactions}
                            className="px-4 py-2 bg-primary-600 hover:bg-primary-500 text-white rounded-lg"
                        >
                            Retry
                        </button>
                    </div>
                ) : transactions.length === 0 ? (
                    <div className="text-center py-20 text-dark-400">
                        No Cast Dex transactions found in the indexer
                    </div>
                ) : (
                    <div className="bg-dark-800/50 rounded-lg overflow-hidden overflow-x-auto">
                        <table className="w-full table-fixed min-w-[900px]">
                            <thead>
                                <tr className="border-b border-dark-700">
                                    <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium w-[180px]">TXID</th>
                                    <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium w-[90px]">Type</th>
                                    <th className="text-right px-4 py-3 text-dark-400 text-sm font-medium w-[120px]">Quantity</th>
                                    <th className="text-right px-4 py-3 text-dark-400 text-sm font-medium w-[100px]">Amount</th>
                                    <th className="text-right px-4 py-3 text-dark-400 text-sm font-medium w-[80px]">Block</th>
                                    <th className="text-center px-4 py-3 text-dark-400 text-sm font-medium w-[80px]">Network</th>
                                    <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium w-[140px]">Date</th>
                                    <th className="text-center px-4 py-3 text-dark-400 text-sm font-medium w-[70px]">Links</th>
                                </tr>
                            </thead>
                            <tbody>
                                {transactions.map((order, index) => {
                                    const opType = getOrderTypeDisplay(order);
                                    return (
                                        <tr 
                                            key={`${order.txid}-${index}`}
                                            className="border-b border-dark-700/50 hover:bg-dark-700/30 transition-colors"
                                        >
                                            <td className="px-4 py-3">
                                                <Link 
                                                    href={`/tx?txid=${order.txid}&from=cast-dex`}
                                                    className="text-primary-400 hover:text-primary-300 font-mono text-xs break-all"
                                                >
                                                    {order.txid?.slice(0, 16)}...{order.txid?.slice(-8)}
                                                </Link>
                                            </td>
                                            <td className="px-4 py-3">
                                                <span className={`inline-flex items-center gap-1 px-2 py-1 rounded text-xs font-medium whitespace-nowrap ${opType.color}`}>
                                                    <span>{opType.icon}</span>
                                                    <span>{opType.type}</span>
                                                </span>
                                            </td>
                                            <td className="px-4 py-3 text-right text-dark-300 text-sm tabular-nums">
                                                {formatTokenQuantity(order.quantity)}
                                            </td>
                                            <td className="px-4 py-3 text-right text-sm tabular-nums">
                                                {order.amount ? (
                                                    <span className="text-orange-400">{order.amount.toLocaleString()} <span className="text-dark-500 text-xs">sats</span></span>
                                                ) : '-'}
                                            </td>
                                            <td className="px-4 py-3 text-right text-dark-300 text-sm tabular-nums">
                                                {order.block_height?.toLocaleString() || '-'}
                                            </td>
                                            <td className="px-4 py-3 text-center">
                                                <span className={`inline-block px-2 py-1 rounded text-xs ${
                                                    order.network === 'mainnet' 
                                                        ? 'bg-orange-500/20 text-orange-400' 
                                                        : 'bg-blue-500/20 text-blue-400'
                                                }`}>
                                                    {order.network || 'testnet4'}
                                                </span>
                                            </td>
                                            <td className="px-4 py-3 text-dark-400 text-sm whitespace-nowrap">
                                                {formatDate(order.created_at)}
                                            </td>
                                            <td className="px-4 py-3">
                                                <div className="flex items-center justify-center gap-1">
                                                    <Link
                                                        href={`/tx?txid=${order.txid}&from=cast-dex`}
                                                        className="px-2 py-1 bg-dark-700 hover:bg-dark-600 text-dark-300 hover:text-white rounded text-xs transition-colors"
                                                        title="View in Explorer"
                                                    >
                                                        TX
                                                    </Link>
                                                    <a
                                                        href={getMempoolUrl(order.txid, order.network)}
                                                        target="_blank"
                                                        rel="noopener noreferrer"
                                                        className="px-2 py-1 bg-dark-700 hover:bg-dark-600 text-dark-300 hover:text-white rounded text-xs transition-colors"
                                                        title="View on Mempool"
                                                    >
                                                        ↗
                                                    </a>
                                                </div>
                                            </td>
                                        </tr>
                                    );
                                })}
                            </tbody>
                        </table>
                    </div>
                )}
            </div>
        </div>
    );
}
