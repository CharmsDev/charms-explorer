'use client';

import { Suspense, useState, useEffect } from 'react';
import { useSearchParams } from 'next/navigation';
import Link from 'next/link';
import { getCharmByTxId, fetchTransactionByTxid } from '@/services/apiServices';
import { getTransaction, isQuickNodeAvailable } from '@/services/quicknodeService';
import { 
    analyzeTransaction, 
    TRANSACTION_TYPES,
    isDexTransaction,
    isBeamingTransaction
} from '@/services/transactions/transactionClassifier';
import { 
    TransactionBadge, 
    TransactionHeader, 
    DexOrderDetails, 
    TokenDetails,
    SpellDataViewer 
} from '@/components/transactions';

const VERIFIED_BRO_HASH = '3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b';

const formatSpellData = (data) => {
    if (!data) return '';

    const replacer = (key, value) => {
        if (Array.isArray(value) && value.length > 4 && value.every(v => typeof v === 'number' && v >= 0 && v <= 255)) {
            const hex = value.map(b => b.toString(16).padStart(2, '0')).join('');
            return `[hex:${hex}]`;
        }
        return value;
    };

    return JSON.stringify(data, replacer, 2);
};

/**
 * Extract token info from spell's app_public_inputs and tx.outs
 * Returns { appId, ticker, amount, decimals } or null
 */
const extractTokenFromSpell = (spellData) => {
    if (!spellData) return null;
    const data = spellData?.native_data || spellData;
    const appInputs = data?.app_public_inputs;
    if (!appInputs) return null;

    // Find token app_id (t/...) in app_public_inputs keys
    const tokenKey = Object.keys(appInputs).find(k => k.startsWith('t/'));
    if (!tokenKey) return null;

    const isBro = tokenKey.includes(VERIFIED_BRO_HASH);

    return {
        appId: tokenKey,
        ticker: isBro ? 'BRO' : null,
        name: isBro ? '$BRO Token' : null,
        icon: isBro ? '🟠' : '🪙',
        isBro,
    };
};

function TransactionPageContent() {
    const searchParams = useSearchParams();
    const txid = searchParams.get('txid');
    const from = searchParams.get('from'); // Source page: transactions, asset, charm, home
    const [charm, setCharm] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
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

                // When coming from transactions list, try /v1/transactions/:txid first
                if (from === 'transactions') {
                    try {
                        const txData = await fetchTransactionByTxid(txid);
                        // Map transaction API response to charm-like shape for display
                        setCharm({
                            txid: txData.txid,
                            block_height: txData.block_height,
                            status: txData.status,
                            confirmations: txData.confirmations,
                            blockchain: txData.blockchain,
                            network: txData.network,
                            date_created: txData.updated_at,
                            data: txData.charm?.native_data || txData.charm,
                            charm: txData.charm,
                            asset_type: txData.charm?.type || 'spell',
                            assets: txData.assets || [],
                            tags: txData.tags,
                            tx_type: txData.tx_type,
                            isTransactionView: true,
                        });
                        setError(null);
                        setLoading(false);
                        return;
                    } catch (txErr) {
                        // Fall through to charm/bitcoin lookup
                    }
                }

                // Try to get charm data first
                try {
                    const data = await getCharmByTxId(txid);
                    setCharm(data);
                    setError(null);
                } catch (charmErr) {
                    // If charm not found, try to get Bitcoin transaction data
                    try {
                        let btcTx;
                        
                        // Try QuickNode first if available
                        if (isQuickNodeAvailable()) {
                            btcTx = await getTransaction(txid);
                        } else {
                            // Fallback to Mempool.space
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
                    const tokenDecimals = charm.decimals || 8;
                    const tokenTicker = charm.ticker || charm.name || null;
                    const formattedQuantity = analysis.orderDetails?.quantity != null
                        ? (analysis.orderDetails.quantity / Math.pow(10, tokenDecimals)).toLocaleString(undefined, { minimumFractionDigits: 0, maximumFractionDigits: 8 })
                        : null;

                    // Determine smart header: detect token transfers from assets or spell
                    const assets = charm.assets || [];
                    const spellTokenInfo = extractTokenFromSpell(charm.spell || charm.data);
                    const isBroTransfer = assets.some(a => a.app_id?.includes(VERIFIED_BRO_HASH)) || spellTokenInfo?.isBro;

                    // Enrich header for BRO transactions (icon + description)
                    let headerType = analysis.type;
                    let headerLabel = null;
                    let headerDescription = null;
                    let headerIcon = null;
                    if (isBroTransfer && (analysis.type === TRANSACTION_TYPES.TOKEN_TRANSFER || analysis.type === TRANSACTION_TYPES.SPELL)) {
                        headerType = TRANSACTION_TYPES.TOKEN_TRANSFER;
                        headerLabel = 'BRO Transfer';
                        headerDescription = 'BRO token transfer on Bitcoin';
                        headerIcon = '🟠';
                    } else if (isBroTransfer && analysis.type === TRANSACTION_TYPES.BRO_MINT) {
                        headerLabel = 'BRO Mint';
                        headerDescription = 'New BRO tokens minted';
                        headerIcon = '🟠';
                    } else if (spellTokenInfo && !analysis.isDex && analysis.type === TRANSACTION_TYPES.SPELL) {
                        headerType = TRANSACTION_TYPES.TOKEN_TRANSFER;
                        headerLabel = 'Token Transfer';
                        headerDescription = 'Charms token transfer';
                    } else if (analysis.isBeaming) {
                        const isBeamOut = analysis.type === TRANSACTION_TYPES.BEAM_OUT;
                        headerIcon = isBeamOut ? '📤' : '📥';
                        headerLabel = isBeamOut ? 'Beam Out' : 'Beam In';
                        headerDescription = isBeamOut
                            ? 'Tokens beamed from Bitcoin to Cardano'
                            : 'Tokens received from Cardano to Bitcoin';
                    }
                    return (
                        <div className="mb-6">
                            {/* Transaction Header with Type */}
                            <div className="card p-6 mb-6">
                                <TransactionHeader
                                    type={headerType}
                                    status="confirmed"
                                    amount={analysis.orderDetails?.asset ? formattedQuantity : null}
                                    ticker={analysis.orderDetails?.asset ? tokenTicker : null}
                                    label={headerLabel}
                                    description={headerDescription}
                                    icon={headerIcon}
                                    beamFlow={analysis.isBeaming ? { isBeamOut: analysis.type === TRANSACTION_TYPES.BEAM_OUT } : null}
                                />
                            </div>

                            {/* TXID Section */}
                            <div className="flex items-center gap-3 mb-2">
                                <h2 className="text-lg font-semibold text-dark-300">TXID</h2>
                                <TransactionBadge type={headerType} size="sm" />
                                {/* Network Badge */}
                                <span className={`px-2 py-1 rounded text-xs font-medium ${
                                    charm.network === 'mainnet'
                                        ? 'bg-orange-500/20 text-orange-400 border border-orange-500/30'
                                        : 'bg-blue-500/20 text-blue-400 border border-blue-500/30'
                                }`}>
                                    ₿ {charm.network === 'mainnet' ? 'Mainnet' : 'Testnet4'}
                                </span>
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
                                        tokenDecimals={tokenDecimals}
                                        tokenTicker={tokenTicker}
                                    />
                                </div>
                            )}

                            {/* Beaming: show commitment hash inline */}
                            {analysis.isBeaming && analysis.type === TRANSACTION_TYPES.BEAM_OUT && (() => {
                                const nd = charm?.data?.native_data || charm?.spell?.native_data || charm?.data;
                                const beamedOuts = nd?.tx?.beamed_outs;
                                if (!beamedOuts) return null;
                                const entries = Object.entries(beamedOuts);
                                // Detect token
                                const appInputs = nd?.app_public_inputs;
                                const tokenKey = appInputs ? Object.keys(appInputs).find(k => k.startsWith('t/')) : null;
                                const isBro = tokenKey?.includes(VERIFIED_BRO_HASH);
                                const tokenLabel = isBro ? 'BRO' : 'tokens';
                                // Total beamed amount
                                const outs = nd?.tx?.outs || [];
                                let totalRaw = 0;
                                entries.forEach(([idx]) => {
                                    const out = outs[parseInt(idx)];
                                    if (out && typeof out === 'object') {
                                        Object.values(out).forEach(v => { if (typeof v === 'number') totalRaw += v; });
                                    }
                                });
                                const totalDisplay = (totalRaw / 1e8).toLocaleString(undefined, { minimumFractionDigits: 0, maximumFractionDigits: 4 });

                                return (
                                    <div className="mt-4 bg-dark-800/50 rounded-lg p-3 border border-cyan-500/20">
                                        <p className="text-sm text-dark-300 mb-2">
                                            <span className="text-cyan-400 font-semibold">{totalDisplay} {tokenLabel}</span> beamed to Cardano via commitment hash
                                        </p>
                                        {entries.map(([idx, hash]) => (
                                            <div key={idx} className="flex items-center gap-2 mt-1">
                                                <span className="text-dark-500 text-xs shrink-0">Commitment</span>
                                                <code className="text-xs text-purple-400 font-mono break-all flex-1">{hash}</code>
                                                <button onClick={() => navigator.clipboard.writeText(hash)} className="p-1 hover:bg-dark-700 rounded transition-colors shrink-0" title="Copy">
                                                    <svg className="w-3.5 h-3.5 text-dark-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                                                    </svg>
                                                </button>
                                            </div>
                                        ))}
                                    </div>
                                );
                            })()}
                        </div>
                    );
                })()}

                {/* Main Content */}
                <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
                    
                    {/* Left Column - Overview */}
                    <div className="lg:col-span-2 space-y-6">
                        {/* Compact Overview */}
                        <div className="card px-4 py-3">
                            <div className="flex flex-wrap items-center gap-x-6 gap-y-1 text-sm">
                                <span className="text-dark-400">Date <span className="text-white font-mono">{new Date(charm.date_created).toLocaleString()}</span></span>
                                <span className="text-dark-400">Block <Link href={`https://mempool.space/block/${charm.block_height}`} target="_blank" className="text-primary-400 hover:text-primary-300 font-mono">{charm.block_height}</Link></span>
                                <span className="px-2 py-0.5 bg-green-900/30 text-green-400 rounded text-xs font-medium">Confirmed</span>
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
                                <div className="flex items-center justify-between mb-4">
                                    <div>
                                        <h2 className="text-xl font-semibold text-white">Spell Data (Raw JSON)</h2>
                                        <p className="text-dark-400 text-xs mt-1">
                                            {charm.spell ? 'Original spell extracted from transaction' : 'Byte arrays are displayed as hex strings for readability'}
                                        </p>
                                    </div>
                                    <button
                                        className="flex items-center gap-1.5 px-3 py-1.5 text-xs bg-dark-800 hover:bg-dark-700 text-dark-300 hover:text-white rounded transition-colors"
                                        onClick={() => {
                                            const text = formatSpellData(charm.spell?.native_data || charm.spell || charm.data);
                                            navigator.clipboard.writeText(text);
                                            setCopied(true);
                                            setTimeout(() => setCopied(false), 2000);
                                        }}
                                    >
                                        {copied ? (
                                            <span className="text-green-400">Copied!</span>
                                        ) : (
                                            <>
                                                <svg xmlns="http://www.w3.org/2000/svg" className="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                                                </svg>
                                                Copy
                                            </>
                                        )}
                                    </button>
                                </div>
                                <div className="bg-dark-900/50 rounded-lg p-4 overflow-x-auto max-h-[600px] overflow-y-auto">
                                    <pre className="text-xs sm:text-sm text-green-400 font-mono whitespace-pre-wrap break-words">
                                        {formatSpellData(charm.spell?.native_data || charm.spell || charm.data)}
                                    </pre>
                                </div>
                            </div>
                        )}
                    </div>

                    {/* Right Column - Related Charm or Bitcoin TX Info */}
                    <div className="space-y-6">
                        {charm.isBitcoinTx ? (
                            <div className="card p-6">
                                <h2 className="text-xl font-semibold text-white mb-6">Transaction Info</h2>
                                <div className="flex flex-col items-center text-center">
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
                                </div>
                            </div>
                        ) : (() => {
                            const assets = charm.assets || [];
                            const tokenInfo = extractTokenFromSpell(charm.spell || charm.data);

                            // Classify assets by role using spell outs data
                            const nativeData = charm.data?.native_data || charm.charm?.native_data || charm.data;
                            const appKeys = nativeData?.app_public_inputs ? Object.keys(nativeData.app_public_inputs) : [];
                            const spellOuts = nativeData?.tx?.outs || [];

                            // Detect which vouts are beamed (from beamed_outs keys)
                            const beamedVouts = new Set();
                            const beamedOuts = nativeData?.tx?.beamed_outs;
                            if (beamedOuts) {
                                Object.keys(beamedOuts).forEach(k => beamedVouts.add(parseInt(k)));
                            }
                            const hasBeamedOuts = beamedVouts.size > 0;

                            // Use role from API if available, fallback to client-side classification
                            const classifyAsset = (asset) => {
                                if (asset.role) return asset.role;
                                // For beam-out: vout in beamed_outs = beamed, others = change
                                if (hasBeamedOuts && asset.asset_type === 'token' && asset.amount > 0) {
                                    return beamedVouts.has(asset.vout) ? 'beamed' : 'change';
                                }
                                if (asset.asset_type === 'token' && asset.amount > 0) return 'output';
                                if (asset.app_id?.startsWith('c/')) return 'contract';
                                if (asset.amount === 0 && !asset.name) return 'contract';
                                return 'output';
                            };

                            const getAssetIcon = (asset) => {
                                if (asset.app_id?.includes(VERIFIED_BRO_HASH)) return '🟠';
                                if (asset.asset_type === 'token') return '🪙';
                                if (asset.app_id?.startsWith('c/')) return '📜';
                                return '⚡';
                            };

                            const getRoleBadge = (role) => {
                                switch (role) {
                                    case 'output': return { label: 'Output', cls: 'bg-green-500/20 text-green-400 border-green-500/30' };
                                    case 'input': return { label: 'Input', cls: 'bg-blue-500/20 text-blue-400 border-blue-500/30' };
                                    case 'beamed': return { label: 'Beamed', cls: 'bg-cyan-500/20 text-cyan-400 border-cyan-500/30' };
                                    case 'change': return { label: 'Change', cls: 'bg-dark-700/50 text-dark-500 border-dark-700' };
                                    case 'contract': return { label: 'Contract', cls: 'bg-purple-500/20 text-purple-400 border-purple-500/30' };
                                    default: return { label: role, cls: 'bg-dark-700 text-dark-300 border-dark-600' };
                                }
                            };

                            if (assets.length > 0) {
                                // Separate primary assets from change
                                const classified = assets.map(a => ({ ...a, _role: classifyAsset(a) }));
                                const primaryAssets = classified.filter(a => a._role !== 'change');
                                const changeAssets = classified.filter(a => a._role === 'change');
                                const decimals = 8;

                                const renderAssetCard = (asset, idx) => {
                                    const badge = getRoleBadge(asset._role);
                                    const icon = getAssetIcon(asset);
                                    const isBro = asset.app_id?.includes(VERIFIED_BRO_HASH);
                                    const displayName = asset.name || (isBro ? 'Bro' : asset.app_id?.startsWith('c/') ? 'Contract' : 'Unknown');
                                    const displaySymbol = asset.symbol || (isBro ? 'BRO' : null);

                                    return (
                                        <div key={idx} className="bg-dark-900/50 rounded-lg p-3 border border-dark-800/50">
                                            <div className="flex items-center justify-between mb-2">
                                                <div className="flex items-center gap-2">
                                                    {asset.image_url ? (
                                                        <img src={asset.image_url} alt={displayName} className="w-7 h-7 rounded-full object-cover" onError={(e) => { e.target.style.display = 'none'; e.target.nextSibling.style.display = 'inline'; }} />
                                                    ) : null}
                                                    <span className="text-lg" style={asset.image_url ? {display:'none'} : {}}>{icon}</span>
                                                    <div>
                                                        <span className="text-white text-sm font-medium">{displayName}</span>
                                                        {displaySymbol && <span className="text-dark-400 text-xs ml-1.5">{displaySymbol}</span>}
                                                    </div>
                                                </div>
                                                <div className="flex items-center gap-2">
                                                    <span className={`text-xs px-1.5 py-0.5 rounded border ${badge.cls}`}>{badge.label}</span>
                                                    <span className="text-dark-500 text-xs font-mono">vout:{asset.vout}</span>
                                                </div>
                                            </div>

                                            {asset.asset_type === 'token' && asset.amount > 0 && (
                                                <div className="flex justify-between items-center mb-1.5">
                                                    <span className="text-dark-400 text-xs">Amount</span>
                                                    <span className="text-white font-mono text-sm">
                                                        {(asset.amount / Math.pow(10, decimals)).toLocaleString(undefined, { minimumFractionDigits: 0, maximumFractionDigits: 8 })}
                                                        {displaySymbol && <span className="text-dark-400 ml-1 text-xs">{displaySymbol}</span>}
                                                    </span>
                                                </div>
                                            )}

                                            {asset.address && (
                                                <div className="text-dark-300 font-mono text-xs break-all mt-1">{asset.address}</div>
                                            )}

                                            <div className="flex items-center gap-2 mt-1.5">
                                                <span className="text-dark-500 text-xs">{asset.asset_type}</span>
                                                {asset.verified && <span className="text-green-500 text-xs">✓ verified</span>}
                                            </div>

                                            {asset.cardano_fingerprint && (
                                                <div className="mt-2 pt-2 border-t border-dark-800/30">
                                                    <div className="flex items-center justify-between">
                                                        <code className="text-xs text-cyan-400 font-mono">{asset.cardano_fingerprint}</code>
                                                        <a href={`https://cardanoscan.io/token/${asset.cardano_fingerprint}`} target="_blank" rel="noopener noreferrer" className="text-cyan-400 hover:text-cyan-300 text-xs flex-shrink-0 ml-2">Cardanoscan</a>
                                                    </div>
                                                </div>
                                            )}
                                        </div>
                                    );
                                };

                                return (
                                    <div className="card p-6">
                                        <h2 className="text-lg font-semibold text-white mb-4">
                                            {hasBeamedOuts ? 'Beamed Assets' : `Assets (${assets.length})`}
                                        </h2>

                                        <div className="space-y-3">
                                            {primaryAssets.map((asset, idx) => renderAssetCard(asset, idx))}
                                        </div>

                                        {/* Change outputs: compact summary */}
                                        {changeAssets.length > 0 && (
                                            <div className="mt-3 pt-3 border-t border-dark-800/30">
                                                <p className="text-dark-500 text-xs mb-1">Change</p>
                                                {changeAssets.map((asset, idx) => {
                                                    const isBro = asset.app_id?.includes(VERIFIED_BRO_HASH);
                                                    const sym = asset.symbol || (isBro ? 'BRO' : '');
                                                    const amt = asset.amount > 0
                                                        ? (asset.amount / Math.pow(10, decimals)).toLocaleString(undefined, { minimumFractionDigits: 0, maximumFractionDigits: 4 })
                                                        : '0';
                                                    return (
                                                        <div key={idx} className="flex items-center justify-between text-dark-500 text-xs py-0.5">
                                                            <span className="font-mono">vout:{asset.vout}</span>
                                                            <span className="font-mono">{amt} {sym}</span>
                                                        </div>
                                                    );
                                                })}
                                            </div>
                                        )}
                                    </div>
                                );
                            }

                            // Fallback: use spell parsing if no assets from API
                            if (!tokenInfo && !charm.name && !charm.app_id) return null;

                            return (
                                <div className="card p-6">
                                    <h2 className="text-lg font-semibold text-white mb-4">Related Asset</h2>
                                    <div className="flex flex-col items-center text-center">
                                        <div className="w-20 h-20 rounded-lg bg-dark-800 flex items-center justify-center mb-3 shadow-lg">
                                            <span className="text-3xl">{tokenInfo?.icon || '⚡'}</span>
                                        </div>
                                        <h3 className="text-lg font-bold text-white mb-1">
                                            {tokenInfo?.name || charm.name || 'Unknown Token'}
                                        </h3>
                                        <span className="text-xs px-2 py-1 rounded-full bg-dark-800 text-dark-300 mb-2 uppercase tracking-wider">
                                            {tokenInfo?.ticker || charm.asset_type || 'TOKEN'}
                                        </span>
                                        {(tokenInfo?.appId || charm.app_id) && (
                                            <div className="w-full pt-3 border-t border-dark-800/50">
                                                <div className="text-dark-400 text-xs mb-1">App ID</div>
                                                <div className="text-dark-300 font-mono text-xs break-all text-left">
                                                    {tokenInfo?.appId || charm.app_id}
                                                </div>
                                            </div>
                                        )}
                                    </div>
                                </div>
                            );
                        })()}
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
