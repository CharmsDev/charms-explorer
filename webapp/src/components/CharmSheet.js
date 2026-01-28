'use client';

import { useState, useEffect } from 'react';
import { X, Copy, ExternalLink, Check } from 'lucide-react';

/**
 * CharmSheet - Modal/Drawer component to display detailed charm information
 * Opens when clicking on any charm (NFT, Token, or regular charm)
 */
export default function CharmSheet({ charm, isOpen, onClose }) {
    const [copied, setCopied] = useState(null);

    // Close on ESC key
    useEffect(() => {
        const handleEsc = (e) => {
            if (e.key === 'Escape') onClose();
        };
        if (isOpen) {
            document.addEventListener('keydown', handleEsc);
            document.body.style.overflow = 'hidden';
        }
        return () => {
            document.removeEventListener('keydown', handleEsc);
            document.body.style.overflow = 'unset';
        };
    }, [isOpen, onClose]);

    if (!isOpen || !charm) return null;

    const copyToClipboard = (text, field) => {
        navigator.clipboard.writeText(text);
        setCopied(field);
        setTimeout(() => setCopied(null), 2000);
    };

    const getCharmTypeLabel = () => {
        if (charm.asset_type === 'nft') return 'NFT';
        if (charm.asset_type === 'token') return 'Token';
        return 'Charm';
    };

    const getCharmTypeColor = () => {
        if (charm.asset_type === 'nft') return 'bg-purple-500';
        if (charm.asset_type === 'token') return 'bg-blue-500';
        return 'bg-gray-500';
    };

    const formatAmount = (amount) => {
        if (!amount) return 'N/A';
        // Assuming 8 decimals for now
        const decimals = 8;
        const value = Number(amount) / Math.pow(10, decimals);
        return value.toLocaleString('en-US', { maximumFractionDigits: decimals });
    };

    const openInExplorer = (txid) => {
        window.open(`https://mempool.space/tx/${txid}`, '_blank');
    };

    return (
        <>
            {/* Backdrop */}
            <div
                className="fixed inset-0 bg-black/50 z-40 transition-opacity"
                onClick={onClose}
            />

            {/* Sheet */}
            <div className="fixed inset-y-0 right-0 w-full max-w-2xl bg-white dark:bg-gray-900 shadow-xl z-50 overflow-y-auto transform transition-transform">
                {/* Header */}
                <div className="sticky top-0 bg-white dark:bg-gray-900 border-b border-gray-200 dark:border-gray-800 p-6 flex items-center justify-between">
                    <div className="flex items-center gap-3">
                        <div className={`px-3 py-1 rounded-full text-white text-sm font-medium ${getCharmTypeColor()}`}>
                            {getCharmTypeLabel()}
                        </div>
                        <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
                            Charm Details
                        </h2>
                    </div>
                    <button
                        onClick={onClose}
                        className="p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
                    >
                        <X className="w-6 h-6 text-gray-500" />
                    </button>
                </div>

                {/* Content */}
                <div className="p-6 space-y-6">
                    {/* Transaction ID */}
                    <div className="space-y-2">
                        <label className="text-sm font-medium text-gray-500 dark:text-gray-400">
                            Transaction ID
                        </label>
                        <div className="flex items-center gap-2">
                            <code className="flex-1 p-3 bg-gray-100 dark:bg-gray-800 rounded-lg text-sm font-mono break-all">
                                {charm.txid}
                            </code>
                            <button
                                onClick={() => copyToClipboard(charm.txid, 'txid')}
                                className="p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
                                title="Copy"
                            >
                                {copied === 'txid' ? (
                                    <Check className="w-5 h-5 text-green-500" />
                                ) : (
                                    <Copy className="w-5 h-5 text-gray-500" />
                                )}
                            </button>
                            <button
                                onClick={() => openInExplorer(charm.txid)}
                                className="p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
                                title="View on Explorer"
                            >
                                <ExternalLink className="w-5 h-5 text-gray-500" />
                            </button>
                        </div>
                    </div>

                    {/* App ID */}
                    <div className="space-y-2">
                        <label className="text-sm font-medium text-gray-500 dark:text-gray-400">
                            App ID (Charm ID)
                        </label>
                        <div className="flex items-center gap-2">
                            <code className="flex-1 p-3 bg-gray-100 dark:bg-gray-800 rounded-lg text-sm font-mono break-all">
                                {charm.app_id}
                            </code>
                            <button
                                onClick={() => copyToClipboard(charm.app_id, 'app_id')}
                                className="p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
                            >
                                {copied === 'app_id' ? (
                                    <Check className="w-5 h-5 text-green-500" />
                                ) : (
                                    <Copy className="w-5 h-5 text-gray-500" />
                                )}
                            </button>
                        </div>
                    </div>

                    {/* Address */}
                    {charm.address && (
                        <div className="space-y-2">
                            <label className="text-sm font-medium text-gray-500 dark:text-gray-400">
                                Holder Address
                            </label>
                            <div className="flex items-center gap-2">
                                <code className="flex-1 p-3 bg-gray-100 dark:bg-gray-800 rounded-lg text-sm font-mono break-all">
                                    {charm.address}
                                </code>
                                <button
                                    onClick={() => copyToClipboard(charm.address, 'address')}
                                    className="p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
                                >
                                    {copied === 'address' ? (
                                        <Check className="w-5 h-5 text-green-500" />
                                    ) : (
                                        <Copy className="w-5 h-5 text-gray-500" />
                                    )}
                                </button>
                            </div>
                        </div>
                    )}

                    {/* Grid of properties */}
                    <div className="grid grid-cols-2 gap-4">
                        {/* Amount */}
                        {charm.amount && (
                            <div className="space-y-2">
                                <label className="text-sm font-medium text-gray-500 dark:text-gray-400">
                                    Amount
                                </label>
                                <div className="p-3 bg-gray-100 dark:bg-gray-800 rounded-lg">
                                    <p className="text-lg font-semibold text-gray-900 dark:text-white">
                                        {formatAmount(charm.amount)}
                                    </p>
                                </div>
                            </div>
                        )}

                        {/* Block Height */}
                        <div className="space-y-2">
                            <label className="text-sm font-medium text-gray-500 dark:text-gray-400">
                                Block Height
                            </label>
                            <div className="p-3 bg-gray-100 dark:bg-gray-800 rounded-lg">
                                <p className="text-lg font-semibold text-gray-900 dark:text-white">
                                    {charm.block_height?.toLocaleString() || 'N/A'}
                                </p>
                            </div>
                        </div>

                        {/* Vout */}
                        <div className="space-y-2">
                            <label className="text-sm font-medium text-gray-500 dark:text-gray-400">
                                Output Index (vout)
                            </label>
                            <div className="p-3 bg-gray-100 dark:bg-gray-800 rounded-lg">
                                <p className="text-lg font-semibold text-gray-900 dark:text-white">
                                    {charm.vout ?? 'N/A'}
                                </p>
                            </div>
                        </div>

                        {/* Spent Status */}
                        <div className="space-y-2">
                            <label className="text-sm font-medium text-gray-500 dark:text-gray-400">
                                Status
                            </label>
                            <div className="p-3 bg-gray-100 dark:bg-gray-800 rounded-lg">
                                <span className={`inline-flex items-center px-3 py-1 rounded-full text-sm font-medium ${
                                    charm.spent 
                                        ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                                        : 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                }`}>
                                    {charm.spent ? 'Spent' : 'Unspent'}
                                </span>
                            </div>
                        </div>

                        {/* Network */}
                        <div className="space-y-2">
                            <label className="text-sm font-medium text-gray-500 dark:text-gray-400">
                                Network
                            </label>
                            <div className="p-3 bg-gray-100 dark:bg-gray-800 rounded-lg">
                                <p className="text-lg font-semibold text-gray-900 dark:text-white capitalize">
                                    {charm.network || 'mainnet'}
                                </p>
                            </div>
                        </div>

                        {/* Blockchain */}
                        <div className="space-y-2">
                            <label className="text-sm font-medium text-gray-500 dark:text-gray-400">
                                Blockchain
                            </label>
                            <div className="p-3 bg-gray-100 dark:bg-gray-800 rounded-lg">
                                <p className="text-lg font-semibold text-gray-900 dark:text-white capitalize">
                                    {charm.blockchain || 'bitcoin'}
                                </p>
                            </div>
                        </div>
                    </div>

                    {/* Charm Data (JSON) */}
                    {charm.data && (
                        <div className="space-y-2">
                            <label className="text-sm font-medium text-gray-500 dark:text-gray-400">
                                Charm Data
                            </label>
                            <div className="relative">
                                <pre className="p-4 bg-gray-100 dark:bg-gray-800 rounded-lg text-sm font-mono overflow-x-auto max-h-96">
                                    {JSON.stringify(charm.data, null, 2)}
                                </pre>
                                <button
                                    onClick={() => copyToClipboard(JSON.stringify(charm.data, null, 2), 'data')}
                                    className="absolute top-2 right-2 p-2 bg-white dark:bg-gray-700 hover:bg-gray-50 dark:hover:bg-gray-600 rounded-lg transition-colors shadow-sm"
                                >
                                    {copied === 'data' ? (
                                        <Check className="w-4 h-4 text-green-500" />
                                    ) : (
                                        <Copy className="w-4 h-4 text-gray-500" />
                                    )}
                                </button>
                            </div>
                        </div>
                    )}

                    {/* Date Created */}
                    {charm.date_created && (
                        <div className="space-y-2">
                            <label className="text-sm font-medium text-gray-500 dark:text-gray-400">
                                Date Created
                            </label>
                            <div className="p-3 bg-gray-100 dark:bg-gray-800 rounded-lg">
                                <p className="text-gray-900 dark:text-white">
                                    {new Date(charm.date_created).toLocaleString()}
                                </p>
                            </div>
                        </div>
                    )}
                </div>

                {/* Footer Actions */}
                <div className="sticky bottom-0 bg-white dark:bg-gray-900 border-t border-gray-200 dark:border-gray-800 p-6">
                    <button
                        onClick={onClose}
                        className="w-full px-6 py-3 bg-gray-900 dark:bg-white text-white dark:text-gray-900 rounded-lg font-medium hover:bg-gray-800 dark:hover:bg-gray-100 transition-colors"
                    >
                        Close
                    </button>
                </div>
            </div>
        </>
    );
}
