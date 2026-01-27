'use client';

import { Suspense, useState, useEffect } from 'react';
import { useSearchParams } from 'next/navigation';
import Link from 'next/link';
import { getCharmByTxId } from '@/services/apiServices';
import { getTransaction, isQuickNodeAvailable } from '@/services/quicknodeService';
import { 
    analyzeTransaction, 
    TRANSACTION_TYPES,
    isDexTransaction 
} from '@/services/transactions/transactionClassifier';
import { 
    TransactionBadge, 
    TransactionHeader, 
    DexOrderDetails, 
    TokenDetails,
    SpellDataViewer 
} from '@/components/transactions';

function TransactionPageContent() {
    const searchParams = useSearchParams();
    const txid = searchParams.get('txid');
    const from = searchParams.get('from'); // Source page: transactions, asset, charm, home
    const [charm, setCharm] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const [activeTab, setActiveTab] = useState('overview');
    const [copied, setCopied] = useState(false);

    // Get back navigation based on source
    const getBackNavigation = () => {
        switch (from) {
            case 'transactions':
                return { href: '/transactions', label: 'Transactions', title: 'Transactions' };
            case 'cast-dex':
                return { href: '/cast-dex', label: 'Cast Dex', title: 'Cast Dex Orders' };
            case 'asset':
                return { href: '/', label: 'Asset', title: 'Asset Details' };
            case 'charm':
                return { href: '/', label: 'Charm', title: 'Charm Details' };
            default:
                return { href: '/', label: 'Explorer', title: 'Explorer' };
        }
    };

    const backNav = getBackNavigation();

    useEffect(() => {
        const loadData = async () => {
            if (!txid) return;

            try {
                setLoading(true);
                
                // Try to get charm data first
                try {
                    const data = await getCharmByTxId(txid);
                    setCharm(data);
                    setError(null);
                } catch (charmErr) {
                    // If charm not found, try to get Bitcoin transaction data
                    console.log('Charm not found, fetching Bitcoin transaction data...');
                    
                    try {
                        let btcTx;
                        
                        // Try QuickNode first if available
                        if (isQuickNodeAvailable()) {
                            console.log('Using QuickNode for transaction data...');
                            btcTx = await getTransaction(txid);
                        } else {
                            // Fallback to Mempool.space
                            console.log('Using Mempool.space for transaction data...');
                            const response = await fetch(`https://mempool.space/api/tx/${txid}`);
                            if (!response.ok) {
                                throw new Error('Transaction not found');
                            }
                            btcTx = await response.json();
                        }
                        
                        // Transform Bitcoin transaction to charm-like format for display
                        setCharm({
                            txid: btcTx.txid,
                            asset_type: 'bitcoin',
                            name: 'Bitcoin Transaction',
                            date_created: new Date(btcTx.status.block_time * 1000).toISOString(),
                            block_height: btcTx.status.block_height,
                            vout: 0,
                            data: btcTx,
                            isBitcoinTx: true, // Flag to identify Bitcoin-only transactions
                        });
                        setError(null);
                    } catch (btcErr) {
                        console.error('Error loading Bitcoin transaction:', btcErr);
                        setError('Transaction not found on blockchain.');
                    }
                }
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
        <div className="min-h-screen">
            {/* Section Header - matches transactions list style */}
            <div className="bg-dark-900/95 backdrop-blur-sm border-b border-dark-800">
                <div className="container mx-auto px-4 py-4">
                    <div className="flex items-center justify-between">
                        <div className="flex items-center gap-4">
                            <Link 
                                href={backNav.href}
                                className="flex items-center gap-2 text-dark-400 hover:text-white transition-colors"
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                                </svg>
                                <span>{backNav.label}</span>
                            </Link>
                            <span className="text-dark-600">|</span>
                            <h1 className="text-xl font-bold text-white">Transaction Details</h1>
                        </div>
                    </div>
                </div>
            </div>

            <div className="container mx-auto px-4 py-6">
                {/* Transaction Info Header */}
                {(() => {
                    const analysis = analyzeTransaction(charm);
                    return (
                        <div className="mb-6">
                            {/* Transaction Header with Type */}
                            <div className="card p-6 mb-6">
                                <TransactionHeader 
                                    type={analysis.type}
                                    status="confirmed"
                                    amount={analysis.orderDetails?.quantity}
                                    ticker={analysis.orderDetails?.asset ? 'tokens' : null}
                                />
                            </div>
                            
                            {/* TXID Section */}
                            <div className="flex items-center gap-3 mb-2">
                                <h2 className="text-lg font-semibold text-dark-300">TXID</h2>
                                <TransactionBadge type={analysis.type} size="sm" />
                            </div>
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
                                    href={`https://mempool.space/${charm.network === 'testnet4' ? 'testnet4/' : ''}tx/${txid}`}
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="ml-2 text-primary-400 hover:text-primary-300 text-xs"
                                >
                                    View on Mempool →
                                </a>
                            </div>
                            
                            {/* DEX Order Details (if applicable) */}
                            {analysis.isDex && analysis.orderDetails && (
                                <div className="mt-6">
                                    <DexOrderDetails 
                                        orderDetails={analysis.orderDetails}
                                        copyToClipboard={(text) => navigator.clipboard.writeText(text)}
                                    />
                                </div>
                            )}
                        </div>
                    );
                })()}

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

                        {/* Bitcoin Transaction Details or Spell Data */}
                        {charm.isBitcoinTx ? (
                            <>
                                {/* Inputs */}
                                <div className="card p-6">
                                    <h2 className="text-xl font-semibold text-white mb-6">
                                        Inputs ({charm.data.vin?.length || 0})
                                    </h2>
                                    <div className="space-y-3">
                                        {charm.data.vin?.map((input, idx) => (
                                            <div key={idx} className="bg-dark-900/50 rounded-lg p-4">
                                                <div className="flex justify-between items-start mb-2">
                                                    <span className="text-dark-400 text-sm">Input #{idx}</span>
                                                    {input.prevout?.value && (
                                                        <span className="text-primary-400 font-mono text-sm">
                                                            {(input.prevout.value / 100000000).toFixed(8)} BTC
                                                        </span>
                                                    )}
                                                </div>
                                                <div className="text-xs text-dark-300 font-mono break-all">
                                                    {input.txid}:{input.vout}
                                                </div>
                                                {input.prevout?.scriptpubkey_address && (
                                                    <div className="text-xs text-dark-400 mt-2">
                                                        From: {input.prevout.scriptpubkey_address}
                                                    </div>
                                                )}
                                            </div>
                                        ))}
                                    </div>
                                </div>

                                {/* Outputs */}
                                <div className="card p-6">
                                    <h2 className="text-xl font-semibold text-white mb-6">
                                        Outputs ({charm.data.vout?.length || 0})
                                    </h2>
                                    <div className="space-y-3">
                                        {charm.data.vout?.map((output, idx) => (
                                            <div key={idx} className="bg-dark-900/50 rounded-lg p-4">
                                                <div className="flex justify-between items-start mb-2">
                                                    <span className="text-dark-400 text-sm">Output #{idx}</span>
                                                    <span className="text-primary-400 font-mono text-sm">
                                                        {(output.value / 100000000).toFixed(8)} BTC
                                                    </span>
                                                </div>
                                                {output.scriptpubkey_address && (
                                                    <div className="text-xs text-dark-300 break-all">
                                                        To: {output.scriptpubkey_address}
                                                    </div>
                                                )}
                                                {output.scriptpubkey_type && (
                                                    <div className="text-xs text-dark-500 mt-1">
                                                        Type: {output.scriptpubkey_type}
                                                    </div>
                                                )}
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            </>
                        ) : (
                            <div className="card p-6">
                                <h2 className="text-xl font-semibold text-white mb-6">Spell Data (Raw JSON)</h2>
                                <p className="text-dark-400 text-xs mb-3">Byte arrays are displayed as hex strings for readability</p>
                                <div className="bg-dark-900/50 rounded-lg p-4 overflow-x-auto max-h-[600px] overflow-y-auto">
                                    <pre className="text-xs sm:text-sm text-green-400 font-mono whitespace-pre-wrap break-words">
                                        {formatSpellData(charm.data)}
                                    </pre>
                                </div>
                            </div>
                        )}
                    </div>

                    {/* Right Column - Related Charm or Bitcoin TX Info */}
                    <div className="space-y-6">
                        <div className="card p-6">
                            <h2 className="text-xl font-semibold text-white mb-6">
                                {charm.isBitcoinTx ? 'Transaction Info' : 'Related Asset'}
                            </h2>
                            
                            <div className="flex flex-col items-center text-center">
                                {charm.isBitcoinTx ? (
                                    <>
                                        <div className="w-24 h-24 rounded-lg bg-dark-800 flex items-center justify-center mb-4 shadow-lg">
                                            <span className="text-3xl">₿</span>
                                        </div>
                                        <h3 className="text-lg font-bold text-white mb-1">Bitcoin Transaction</h3>
                                        <span className="text-xs px-2 py-1 rounded-full bg-dark-800 text-dark-300 mb-4 uppercase tracking-wider">
                                            BTC Transfer
                                        </span>
                                        <div className="w-full space-y-2 text-sm">
                                            <div className="flex justify-between">
                                                <span className="text-dark-400">Size</span>
                                                <span className="text-white">{charm.data.size} bytes</span>
                                            </div>
                                            <div className="flex justify-between">
                                                <span className="text-dark-400">Weight</span>
                                                <span className="text-white">{charm.data.weight} WU</span>
                                            </div>
                                            <div className="flex justify-between">
                                                <span className="text-dark-400">Fee</span>
                                                <span className="text-white">{(charm.data.fee / 100000000).toFixed(8)} BTC</span>
                                            </div>
                                            <div className="flex justify-between">
                                                <span className="text-dark-400">Fee Rate</span>
                                                <span className="text-white">{(charm.data.fee / charm.data.weight * 4).toFixed(2)} sat/vB</span>
                                            </div>
                                        </div>
                                    </>
                                ) : (
                                    <>
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

                                        <div className="mt-6 pt-6 border-t border-dark-800/50 w-full">
                                            <div className="flex justify-between text-sm mb-2">
                                                <span className="text-dark-400">Vout</span>
                                                <span className="text-white font-mono">{charm.vout}</span>
                                            </div>
                                            <div className="flex justify-between text-sm">
                                                <span className="text-dark-400">Charm ID</span>
                                                <span className="text-white font-mono truncate w-32" title={charm.charmid}>{charm.charmid}</span>
                                            </div>
                                        </div>
                                    </>
                                )}
                            </div>
                        </div>
                    </div>

                </div>
            </div>
        </div>
    );
}

// Wrapper with Suspense boundary for useSearchParams
export default function TransactionPage() {
    return (
        <Suspense fallback={
            <div className="container mx-auto px-4 py-12 flex justify-center">
                <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-500"></div>
            </div>
        }>
            <TransactionPageContent />
        </Suspense>
    );
}
