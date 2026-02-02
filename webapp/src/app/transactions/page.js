'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';
import { fetchAssets } from '@/services/apiServices';
import { 
    classifyTransaction, 
    getTransactionLabel, 
    getTransactionColors 
} from '@/services/transactions/transactionClassifier';

export default function TransactionsPage() {
    const [transactions, setTransactions] = useState([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const [page, setPage] = useState(1);
    const [totalPages, setTotalPages] = useState(1);
    const [total, setTotal] = useState(0);
    const ITEMS_PER_PAGE = 50;

    useEffect(() => {
        loadTransactions();
    }, [page]);

    const loadTransactions = async () => {
        try {
            setLoading(true);
            const result = await fetchAssets(page, ITEMS_PER_PAGE, 'newest');
            setTransactions(result.assets || []);
            setTotal(result.total || 0);
            setTotalPages(result.totalPages || 1);
            setError(null);
        } catch (err) {
            setError('Failed to load transactions');
        } finally {
            setLoading(false);
        }
    };

    const getMempoolUrl = (txid, network) => {
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
                                className="px-4 py-2 rounded-lg text-sm font-medium bg-primary-600 text-white transition-all"
                            >
                                Transactions
                            </Link>
                            <Link 
                                href="/cast-dex"
                                className="px-4 py-2 rounded-lg text-sm font-medium bg-dark-800 text-dark-300 hover:bg-dark-700 hover:text-white transition-all"
                            >
                                Cast Dex
                            </Link>
                        </div>
                        <div className="text-dark-400">
                            <span className="text-primary-400 font-semibold">{total.toLocaleString()}</span> transactions
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
                    <div className="text-center py-20 text-red-400">{error}</div>
                ) : (
                    <>
                        <div className="bg-dark-800/50 rounded-lg overflow-x-auto">
                            <table className="w-full min-w-[900px]">
                                <thead>
                                    <tr className="border-b border-dark-700">
                                        <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium whitespace-nowrap">TXID</th>
                                        <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium whitespace-nowrap">Type</th>
                                        <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium whitespace-nowrap">Asset</th>
                                        <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium whitespace-nowrap">Block</th>
                                        <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium whitespace-nowrap">Network</th>
                                        <th className="text-left px-4 py-3 text-dark-400 text-sm font-medium whitespace-nowrap">Date</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {transactions.map((tx, index) => {
                                        const txType = classifyTransaction(tx);
                                        const colors = getTransactionColors(txType);
                                        return (
                                            <tr 
                                                key={`${tx.txid}-${index}`}
                                                className="border-b border-dark-700/50 hover:bg-dark-700/30 transition-colors"
                                            >
                                                <td className="px-4 py-3">
                                                    <Link 
                                                        href={`/tx?txid=${tx.txid}&from=transactions`}
                                                        className="text-primary-400 hover:text-primary-300 font-mono text-xs whitespace-nowrap"
                                                    >
                                                        {tx.txid}
                                                    </Link>
                                                </td>
                                                <td className="px-4 py-3">
                                                    <span className={`px-2 py-1 rounded text-xs font-medium whitespace-nowrap ${colors.bg} ${colors.text}`}>
                                                        {getTransactionLabel(txType)}
                                                    </span>
                                                </td>
                                                <td className="px-4 py-3">
                                                    <Link 
                                                        href={`/asset/${tx.id}`}
                                                        className="text-white hover:text-primary-300 text-sm whitespace-nowrap"
                                                    >
                                                        {tx.name || `Charm ${tx.app_id?.substring(0, 8) || ''}` || '-'}
                                                    </Link>
                                                </td>
                                                <td className="px-4 py-3 text-dark-300 text-sm whitespace-nowrap">
                                                    {tx.block_height?.toLocaleString() || '-'}
                                                </td>
                                                <td className="px-4 py-3">
                                                    <span className={`px-2 py-1 rounded text-xs whitespace-nowrap ${
                                                        tx.network === 'mainnet' 
                                                            ? 'bg-orange-500/20 text-orange-400' 
                                                            : 'bg-blue-500/20 text-blue-400'
                                                    }`}>
                                                        {tx.network || 'testnet4'}
                                                    </span>
                                                </td>
                                                <td className="px-4 py-3 text-dark-400 text-sm whitespace-nowrap">
                                                    {formatDate(tx.date_created)}
                                                </td>
                                            </tr>
                                        );
                                    })}
                                </tbody>
                            </table>
                        </div>

                        {/* Pagination */}
                        {totalPages > 1 && (
                            <div className="flex justify-center items-center gap-4 mt-6">
                                <button
                                    onClick={() => setPage(p => Math.max(1, p - 1))}
                                    disabled={page === 1}
                                    className="px-4 py-2 bg-dark-800 text-white rounded-lg disabled:opacity-50 disabled:cursor-not-allowed hover:bg-dark-700 transition-colors"
                                >
                                    Previous
                                </button>
                                <span className="text-dark-400">
                                    Page {page} of {totalPages}
                                </span>
                                <button
                                    onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                                    disabled={page === totalPages}
                                    className="px-4 py-2 bg-dark-800 text-white rounded-lg disabled:opacity-50 disabled:cursor-not-allowed hover:bg-dark-700 transition-colors"
                                >
                                    Next
                                </button>
                            </div>
                        )}
                    </>
                )}
            </div>
        </div>
    );
}
