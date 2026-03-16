'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import Link from 'next/link';
import { fetchAllDexOrders } from '@/services/api/dex';
import { useNetwork } from '@/context/NetworkContext';
import { useAutoRefresh } from '@/hooks/useAutoRefresh';

const CAST_APP_URL = process.env.NEXT_PUBLIC_CAST_APP_URL || 'https://cast.charms.dev';

// BRO token uses 8 decimal places (10^8)
const TOKEN_DECIMALS = 8;

const formatTokenQuantity = (rawQuantity) => {
    if (!rawQuantity) return '-';
    const displayValue = rawQuantity / Math.pow(10, TOKEN_DECIMALS);
    return displayValue.toLocaleString(undefined, {
        minimumFractionDigits: 0,
        maximumFractionDigits: 8
    });
};

// Combine side + status into a descriptive activity type
const getActivityType = (order) => {
    const side = order.side?.toLowerCase();
    const status = order.status?.toLowerCase();

    if (status === 'cancelled') {
        return { label: 'Cancel', color: 'bg-red-500/20 text-red-400', icon: '🚫' };
    }
    if (status === 'filled') {
        if (side === 'ask') return { label: 'Filled Ask', color: 'bg-purple-500/20 text-purple-400', icon: '✅' };
        if (side === 'bid') return { label: 'Filled Bid', color: 'bg-purple-500/20 text-purple-400', icon: '✅' };
        return { label: 'Filled', color: 'bg-purple-500/20 text-purple-400', icon: '✅' };
    }
    if (status === 'partial') {
        if (side === 'ask') return { label: 'Partial Ask', color: 'bg-yellow-500/20 text-yellow-400', icon: '⚡' };
        if (side === 'bid') return { label: 'Partial Bid', color: 'bg-yellow-500/20 text-yellow-400', icon: '⚡' };
        return { label: 'Partial', color: 'bg-yellow-500/20 text-yellow-400', icon: '⚡' };
    }
    // open = create
    if (side === 'ask') return { label: 'Create Ask', color: 'bg-green-500/20 text-green-400', icon: '📈' };
    if (side === 'bid') return { label: 'Create Bid', color: 'bg-blue-500/20 text-blue-400', icon: '📉' };
    return { label: 'Order', color: 'bg-dark-500/20 text-dark-400', icon: '🔄' };
};

// Format price from price_per_token (API already returns sats * 10^8 / den)
const formatPrice = (order) => {
    if (!order.price_per_token || order.price_per_token === 0) return '-';
    return Math.round(order.price_per_token).toLocaleString();
};

export default function CastDexPage() {
    const { getNetworkParam, isHydrated } = useNetwork();
    const [transactions, setTransactions] = useState([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);

    const loadCastTransactions = useCallback(async () => {
        try {
            setLoading(true);
            const networkParam = getNetworkParam();
            const result = await fetchAllDexOrders(networkParam);
            setTransactions(result?.orders || []);
            setError(null);
        } catch (err) {
            setError('Failed to load Cast Dex activity from indexer.');
        } finally {
            setLoading(false);
        }
    }, [getNetworkParam]);

    const transactionsRef = useRef(transactions);
    transactionsRef.current = transactions;

    const silentRefresh = useCallback(async () => {
        const networkParam = getNetworkParam();
        const result = await fetchAllDexOrders(networkParam);
        const freshOrders = result?.orders || [];
        if (freshOrders.length === 0) return;

        const existingKeys = new Set(
            transactionsRef.current.map(o => `${o.txid}:${o.vout}`)
        );
        const newOrders = freshOrders.filter(o => !existingKeys.has(`${o.txid}:${o.vout}`));

        if (newOrders.length > 0) {
            console.log(`[Cast DEX] +${newOrders.length} new order(s)`);
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
                                <span className="text-primary-400 font-semibold">{transactions.length}</span> events
                            </span>
                            <span
                                className="px-3 py-1.5 bg-dark-700/50 text-dark-500 rounded-lg text-sm font-medium cursor-default"
                                title="Coming soon"
                            >
                                Order Book →
                            </span>
                        </div>
                    </div>
                </div>
            </div>

            {/* Activity Feed */}
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
                        No Cast Dex activity found
                    </div>
                ) : (
                    <div className="bg-dark-800/50 rounded-lg overflow-hidden overflow-x-auto">
                        <table className="w-full table-fixed min-w-[950px]">
                            <thead>
                                <tr className="border-b border-dark-700">
                                    <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium w-[110px]">Type</th>
                                    <th className="text-right px-4 py-3 text-dark-400 text-sm font-medium w-[90px]">Quantity</th>
                                    <th className="text-right px-4 py-3 text-dark-400 text-sm font-medium w-[110px]">Price</th>
                                    <th className="text-right px-4 py-3 text-dark-400 text-sm font-medium w-[90px]">Amount</th>
                                    <th className="text-right px-4 py-3 text-dark-400 text-sm font-medium w-[80px]">Block</th>
                                    <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium w-[150px]">Date</th>
                                    <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium w-[180px]">TXID</th>
                                    <th className="text-center px-4 py-3 text-dark-400 text-sm font-medium w-[70px]">Links</th>
                                </tr>
                            </thead>
                            <tbody>
                                {transactions.map((order, index) => {
                                    const activity = getActivityType(order);
                                    return (
                                        <tr
                                            key={`${order.txid}-${index}`}
                                            className="border-b border-dark-700/50 hover:bg-dark-700/30 transition-colors"
                                        >
                                            <td className="px-4 py-3">
                                                <span className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded text-xs font-medium whitespace-nowrap ${activity.color}`}>
                                                    <span>{activity.icon}</span>
                                                    <span>{activity.label}</span>
                                                </span>
                                            </td>
                                            <td className="px-4 py-3 text-right text-white text-sm font-mono tabular-nums">
                                                {formatTokenQuantity(order.quantity)}
                                                <span className="text-dark-500 text-xs ml-1">BRO</span>
                                            </td>
                                            <td className="px-4 py-3 text-right text-sm tabular-nums">
                                                <span className="text-dark-300 font-mono">{formatPrice(order)}</span>
                                                <span className="text-dark-600 text-xs ml-1">sats/BRO</span>
                                            </td>
                                            <td className="px-4 py-3 text-right text-sm tabular-nums">
                                                {order.amount ? (
                                                    <span className="text-orange-400 font-mono">{order.amount.toLocaleString()}</span>
                                                ) : '-'}
                                            </td>
                                            <td className="px-4 py-3 text-right text-sm tabular-nums">
                                                {order.block_height ? (
                                                    <span className="text-dark-300">{order.block_height.toLocaleString()}</span>
                                                ) : (
                                                    <a
                                                        href={getMempoolUrl(order.txid, order.network)}
                                                        target="_blank"
                                                        rel="noopener noreferrer"
                                                        className="px-2 py-1 rounded text-xs font-medium bg-yellow-500/20 text-yellow-400 hover:bg-yellow-500/30 transition-colors"
                                                    >
                                                        mempool
                                                    </a>
                                                )}
                                            </td>
                                            <td className="px-4 py-3 text-dark-400 text-sm whitespace-nowrap">
                                                {formatDate(order.created_at)}
                                            </td>
                                            <td className="px-4 py-3">
                                                <Link
                                                    href={`/tx?txid=${order.txid}&from=cast-dex`}
                                                    className="text-primary-400 hover:text-primary-300 font-mono text-xs"
                                                >
                                                    {order.txid?.slice(0, 16)}...{order.txid?.slice(-8)}
                                                </Link>
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
