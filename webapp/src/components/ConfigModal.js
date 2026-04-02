'use client';

import { useState } from 'react';
import { API_BASE_URL } from '../services/apiConfig';
import { version as explorerVersion } from '../../package.json';

const QUICKNODE_URL = process.env.NEXT_PUBLIC_QUICKNODE_BITCOIN_MAINNET_URL || '';

const DATA_SOURCES = [
    { data: 'Charms', source: 'Explorer API', badge: 'explorer', endpoint: '/v1/charms' },
    { data: 'Transactions', source: 'Explorer API', badge: 'explorer', endpoint: '/v1/transactions' },
    { data: 'Assets', source: 'Explorer API', badge: 'explorer', endpoint: '/v1/assets' },
    { data: 'DEX Orders', source: 'Explorer API', badge: 'explorer', endpoint: '/v1/dex/orders' },
    { data: 'Wallet UTXOs', source: 'Explorer API', badge: 'explorer', endpoint: '/v1/wallet/utxos/<addr>' },
    { data: 'Wallet Balance', source: 'Explorer API', badge: 'explorer', endpoint: '/v1/wallet/balance/<addr>' },
    { data: 'Token Balances', source: 'Explorer API', badge: 'explorer', endpoint: '/v1/wallet/charms/<addr>' },
    { data: 'Fee Estimates', source: 'Explorer API', badge: 'explorer', endpoint: '/v1/wallet/fee-estimate' },
    { data: 'Broadcast TX', source: 'Explorer → Maestro → QuickNode', badge: 'multi', endpoint: '/v1/wallet/broadcast (failover)' },
    { data: 'TX Lookup', source: 'Explorer → Maestro → QuickNode', badge: 'multi', endpoint: '/v1/wallet/tx/<txid> (failover)' },
];

function truncate(str, len = 20) {
    if (!str) return '—';
    if (str.length <= len * 2) return str;
    return `${str.slice(0, len)}...${str.slice(-len)}`;
}

export default function ConfigModal({ isOpen, onClose }) {
    const [activeTab, setActiveTab] = useState('config');

    if (!isOpen) return null;

    const explorerApiUrl = API_BASE_URL || '—';
    const hasQuickNode = !!QUICKNODE_URL;

    const badgeClasses = {
        explorer: 'bg-blue-500/15 text-blue-400',
        multi: 'bg-emerald-500/15 text-emerald-400',
    };

    return (
        <div
            className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-[1000] p-4"
            onClick={onClose}
        >
            <div
                className="bg-dark-900 border border-dark-700 rounded-2xl w-full max-w-[720px] max-h-[90vh] overflow-y-auto shadow-2xl"
                onClick={(e) => e.stopPropagation()}
            >
                {/* Header */}
                <div className="flex justify-between items-center px-6 py-5 border-b border-dark-700">
                    <h2 className="text-lg font-bold text-white">Explorer Configuration</h2>
                    <button
                        onClick={onClose}
                        className="text-dark-400 hover:text-white hover:bg-dark-800 rounded-lg p-1 transition-colors"
                    >
                        <svg className="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                </div>

                {/* Tabs */}
                <div className="flex border-b border-dark-700 px-6">
                    <button
                        className={`px-4 py-2.5 text-sm font-semibold border-b-2 -mb-px transition-colors ${activeTab === 'config' ? 'text-white border-primary-500' : 'text-dark-400 border-transparent hover:text-white'}`}
                        onClick={() => setActiveTab('config')}
                    >
                        Configuration
                    </button>
                    <button
                        className={`px-4 py-2.5 text-sm font-semibold border-b-2 -mb-px transition-colors ${activeTab === 'data-sources' ? 'text-white border-primary-500' : 'text-dark-400 border-transparent hover:text-white'}`}
                        onClick={() => setActiveTab('data-sources')}
                    >
                        Data Sources
                    </button>
                </div>

                {/* Content */}
                <div className="p-6">
                    {activeTab === 'config' && (
                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
                            {/* Left column */}
                            <div className="space-y-6">
                                {/* General */}
                                <div>
                                    <h3 className="text-xs font-semibold uppercase tracking-wider text-dark-400 mb-3 pb-2 border-b border-dark-700">General</h3>
                                    <div className="space-y-3">
                                        <div className="flex items-start gap-3">
                                            <svg className="h-4 w-4 text-primary-400 mt-0.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
                                            </svg>
                                            <div>
                                                <span className="text-xs text-dark-400">Charms Version</span>
                                                <span className="block text-sm font-semibold text-purple-400">v12</span>
                                            </div>
                                        </div>
                                        <div className="flex items-start gap-3">
                                            <svg className="h-4 w-4 text-primary-400 mt-0.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                                            </svg>
                                            <div>
                                                <span className="text-xs text-dark-400">Explorer Version</span>
                                                <span className="block text-sm font-semibold text-emerald-400">v{explorerVersion}</span>
                                            </div>
                                        </div>
                                        <div className="flex items-start gap-3">
                                            <svg className="h-4 w-4 text-primary-400 mt-0.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
                                            </svg>
                                            <div>
                                                <span className="text-xs text-dark-400">Network</span>
                                                <span className="block text-sm font-medium text-emerald-400">MAINNET</span>
                                            </div>
                                        </div>
                                    </div>
                                </div>

                                {/* APIs */}
                                <div>
                                    <h3 className="text-xs font-semibold uppercase tracking-wider text-dark-400 mb-3 pb-2 border-b border-dark-700">APIs</h3>
                                    <div className="space-y-3">
                                        <div className="flex items-start gap-3">
                                            <svg className="h-4 w-4 text-primary-400 mt-0.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2" />
                                            </svg>
                                            <div className="min-w-0">
                                                <span className="text-xs text-dark-400">Explorer API</span>
                                                <span className="block text-xs font-mono text-dark-200 break-all">{explorerApiUrl}</span>
                                            </div>
                                        </div>
                                        <div className="flex items-start gap-3">
                                            <svg className="h-4 w-4 text-primary-400 mt-0.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                                            </svg>
                                            <div className="min-w-0">
                                                <span className="text-xs text-dark-400">Maestro (primary)</span>
                                                <span className="block text-xs font-mono text-dark-200">xbt-mainnet.gomaestro-api.org</span>
                                            </div>
                                        </div>
                                        <div className="flex items-start gap-3">
                                            <svg className="h-4 w-4 text-primary-400 mt-0.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                                            </svg>
                                            <div className="min-w-0">
                                                <span className="text-xs text-dark-400">QuickNode (fallback)</span>
                                                <span className="block text-xs font-mono text-dark-200 break-all">{hasQuickNode ? truncate(QUICKNODE_URL, 22) : '—'}</span>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>

                            {/* Right column */}
                            <div className="space-y-6">
                                {/* Contract IDs */}
                                <div>
                                    <h3 className="text-xs font-semibold uppercase tracking-wider text-dark-400 mb-3 pb-2 border-b border-dark-700">Contract IDs</h3>
                                    <div className="space-y-3">
                                        <div>
                                            <span className="text-xs text-dark-400">DEX App ID</span>
                                            <span className="block text-xs font-mono text-dark-200 break-all">
                                                {truncate('b/0000000000000000000000000000000000000000000000000000000000000000/a471d3fcc436ae7cbc0e0c82a68cdc8e003ee21ef819e1acf834e11c43ce47d8', 24)}
                                            </span>
                                        </div>
                                        <div>
                                            <span className="text-xs text-dark-400">Token (BRO)</span>
                                            <span className="block text-xs font-mono text-dark-200 break-all">
                                                {truncate('t/3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b/c975d4e0c292fb95efbda5c13312d6ac1d8b5aeff7f0f1e5578645a2da70ff5f', 24)}
                                            </span>
                                        </div>
                                    </div>
                                </div>

                                {/* Addresses */}
                                <div>
                                    <h3 className="text-xs font-semibold uppercase tracking-wider text-dark-400 mb-3 pb-2 border-b border-dark-700">Addresses</h3>
                                    <div className="space-y-3">
                                        <div>
                                            <span className="text-xs text-dark-400">DEX Fee Address</span>
                                            <span className="block text-xs font-mono text-dark-200 break-all">bc1qxxxjm06n50uugxewxe5r5w5tskqwq4gkwrm0al</span>
                                        </div>
                                    </div>
                                </div>

                                {/* Data Providers */}
                                <div>
                                    <h3 className="text-xs font-semibold uppercase tracking-wider text-dark-400 mb-3 pb-2 border-b border-dark-700">Data Providers</h3>
                                    <div className="space-y-3">
                                        <div className="flex items-start gap-3">
                                            <svg className="h-4 w-4 text-primary-400 mt-0.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" />
                                            </svg>
                                            <div>
                                                <span className="text-xs text-dark-400">Bitcoin Blockchain</span>
                                                <span className="block text-xs text-dark-200">Maestro API (primary)</span>
                                                <span className="block text-xs text-dark-300">QuickNode RPC (fallback)</span>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    )}

                    {activeTab === 'data-sources' && (
                        <div className="overflow-x-auto">
                            <table className="w-full text-sm">
                                <thead>
                                    <tr className="border-b border-dark-700">
                                        <th className="text-left text-xs font-semibold uppercase tracking-wider text-dark-400 py-2 px-3">Data</th>
                                        <th className="text-left text-xs font-semibold uppercase tracking-wider text-dark-400 py-2 px-3">Source</th>
                                        <th className="text-left text-xs font-semibold uppercase tracking-wider text-dark-400 py-2 px-3">Endpoint</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {DATA_SOURCES.map((ds) => (
                                        <tr key={ds.data} className="border-b border-dark-800/50">
                                            <td className="py-2.5 px-3 text-white font-medium">{ds.data}</td>
                                            <td className="py-2.5 px-3">
                                                <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-semibold ${badgeClasses[ds.badge]}`}>
                                                    {ds.source}
                                                </span>
                                            </td>
                                            <td className="py-2.5 px-3 font-mono text-xs text-dark-400 break-all">{ds.endpoint}</td>
                                        </tr>
                                    ))}
                                </tbody>
                            </table>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
