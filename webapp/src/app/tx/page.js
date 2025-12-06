'use client';

import { useState, useEffect } from 'react';
import { useSearchParams } from 'next/navigation';
import Link from 'next/link';
import { getCharmByTxId } from '@/services/apiServices';

export default function TransactionPage() {
    const searchParams = useSearchParams();
    const txid = searchParams.get('txid');
    const [charm, setCharm] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const [activeTab, setActiveTab] = useState('overview');
    const [copied, setCopied] = useState(false);

    useEffect(() => {
        const loadData = async () => {
            if (!txid) return;

            try {
                setLoading(true);
                const data = await getCharmByTxId(txid);
                setCharm(data);
                setError(null);
            } catch (err) {
                console.error('Error loading transaction:', err);
                setError('Transaction not found or error loading data.');
            } finally {
                setLoading(false);
            }
        };

        loadData();
    }, [txid]);

    if (!txid) {
        return (
            <div className="container mx-auto px-4 py-12 text-center text-dark-400">
                No transaction ID provided.
            </div>
        );
    }

    if (loading) {
        return (
            <div className="container mx-auto px-4 py-12 flex justify-center">
                <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-500"></div>
            </div>
        );
    }

    if (error || !charm) {
        return (
            <div className="container mx-auto px-4 py-12 text-center">
                <div className="text-red-400 mb-4">{error || 'Transaction not found'}</div>
                <Link href="/" className="text-primary-400 hover:text-primary-300">
                    &larr; Back to Home
                </Link>
            </div>
        );
    }

    return (
        <div className="min-h-screen pb-20 pt-24">
            <div className="container mx-auto px-4">
                {/* Header */}
                <div className="mb-8">
                    <Link href="/" className="text-dark-400 hover:text-white mb-4 inline-block transition-colors">
                        &larr; Back to Explorer
                    </Link>
                    <h1 className="text-3xl font-bold gradient-text mb-2">Transaction Details</h1>
                    <div className="flex items-center text-dark-400 text-sm break-all">
                        <span className="font-mono">{txid}</span>
                        <button 
                            className="ml-2 text-dark-500 hover:text-primary-400 transition-colors relative"
                            onClick={() => {
                                navigator.clipboard.writeText(txid);
                                setCopied(true);
                                setTimeout(() => setCopied(false), 2000);
                            }}
                            title="Copy TXID"
                        >
                            {copied ? (
                                <span className="text-green-400 text-xs">✓ Copied!</span>
                            ) : (
                                <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                                </svg>
                            )}
                        </button>
                        <a 
                            href={`https://mempool.space/tx/${txid}`}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="ml-2 text-primary-400 hover:text-primary-300 text-xs"
                        >
                            View on Mempool →
                        </a>
                    </div>
                </div>

                {/* Main Content */}
                <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
                    
                    {/* Left Column - Overview */}
                    <div className="lg:col-span-2 space-y-6">
                        <div className="card p-6">
                            <h2 className="text-xl font-semibold text-white mb-6">Overview</h2>
                            
                            <div className="space-y-4">
                                <div className="flex flex-col sm:flex-row justify-between border-b border-dark-800/50 pb-4">
                                    <span className="text-dark-400 mb-1 sm:mb-0">Date Created</span>
                                    <span className="text-white font-mono">{new Date(charm.date_created).toLocaleString()}</span>
                                </div>
                                
                                <div className="flex flex-col sm:flex-row justify-between border-b border-dark-800/50 pb-4">
                                    <span className="text-dark-400 mb-1 sm:mb-0">Block Height</span>
                                    <Link href={`https://mempool.space/block/${charm.block_height}`} target="_blank" className="text-primary-400 hover:text-primary-300 font-mono">
                                        {charm.block_height}
                                    </Link>
                                </div>

                                <div className="flex flex-col sm:flex-row justify-between border-b border-dark-800/50 pb-4">
                                    <span className="text-dark-400 mb-1 sm:mb-0">Status</span>
                                    <span className="px-2 py-1 bg-green-900/30 text-green-400 rounded text-xs font-medium w-fit">
                                        Confirmed
                                    </span>
                                </div>
                            </div>
                        </div>

                        {/* Spell / Charm Data */}
                        <div className="card p-6">
                            <h2 className="text-xl font-semibold text-white mb-6">Spell Data (Raw JSON)</h2>
                            <div className="bg-dark-900/50 rounded-lg p-4 overflow-x-auto">
                                <pre className="text-xs sm:text-sm text-green-400 font-mono whitespace-pre-wrap break-all">
                                    {JSON.stringify(charm.data, null, 2)}
                                </pre>
                            </div>
                        </div>
                    </div>

                    {/* Right Column - Related Charm */}
                    <div className="space-y-6">
                        <div className="card p-6">
                            <h2 className="text-xl font-semibold text-white mb-6">Related Asset</h2>
                            
                            <div className="flex flex-col items-center text-center">
                                {charm.image ? (
                                    <img src={charm.image} alt={charm.name} className="w-24 h-24 rounded-lg object-cover mb-4 shadow-lg" />
                                ) : (
                                    <div className="w-24 h-24 rounded-lg bg-dark-800 flex items-center justify-center mb-4 shadow-lg">
                                        <span className="text-3xl">
                                            {charm.asset_type === 'nft' ? '🎨' : 
                                             charm.asset_type === 'token' ? '🪙' : 
                                             charm.asset_type === 'dapp' ? '⚙️' :
                                             charm.asset_type === 'other' ? '📦' : '⚡'}
                                        </span>
                                    </div>
                                )}
                                
                                <h3 className="text-lg font-bold text-white mb-1">{charm.name || 'Unnamed Asset'}</h3>
                                <span className="text-xs px-2 py-1 rounded-full bg-dark-800 text-dark-300 mb-4 uppercase tracking-wider">
                                    {charm.asset_type}
                                </span>

                                <Link 
                                    href={`/asset?appid=${encodeURIComponent(charm.app_id || charm.charmid)}`}
                                    className="btn btn-primary w-full justify-center"
                                >
                                    View Asset Details
                                </Link>
                            </div>

                            <div className="mt-6 pt-6 border-t border-dark-800/50">
                                <div className="flex justify-between text-sm mb-2">
                                    <span className="text-dark-400">Vout</span>
                                    <span className="text-white font-mono">{charm.vout}</span>
                                </div>
                                <div className="flex justify-between text-sm">
                                    <span className="text-dark-400">Charm ID</span>
                                    <span className="text-white font-mono truncate w-32" title={charm.charmid}>{charm.charmid}</span>
                                </div>
                            </div>
                        </div>
                    </div>

                </div>
            </div>
        </div>
    );
}
