'use client';

/**
 * Beaming Details Component
 * Displays detailed information about Beaming transactions (hash-locked token transfers).
 *
 * Beaming is a mechanism for transferring tokens between addresses without revealing
 * the recipient upfront. The sender locks tokens to a commitment hash. The recipient
 * later claims them by providing the matching preimage in an "unbeam" transaction.
 *
 * Flow: Sender → Lock tokens to hash → Recipient provides preimage → Tokens released
 *
 * [RJJ-BEAMING] This component handles the "beam" direction (sending/locking tokens).
 * TODO: [RJJ-UNBEAM] In the future, "unbeam" transactions will reverse this process.
 */

const TOKEN_DECIMALS = 8;
const VERIFIED_BRO_HASH = '3d7fe7e4cea6121947af73d70e5119bebd8aa5b7edfe74bfaf6e779a1847bd9b';

const formatTokenAmount = (rawAmount) => {
    if (rawAmount === undefined || rawAmount === null) return '-';
    const displayValue = rawAmount / Math.pow(10, TOKEN_DECIMALS);
    return displayValue.toLocaleString(undefined, {
        minimumFractionDigits: 0,
        maximumFractionDigits: 4
    });
};

/**
 * Detect which token is being beamed from app_public_inputs keys.
 * Returns { name, appId } or null.
 */
function detectBeamedToken(nativeData) {
    const appInputs = nativeData?.app_public_inputs;
    if (!appInputs) return null;

    for (const key of Object.keys(appInputs)) {
        if (key.startsWith('t/')) {
            // Check if this is BRO
            if (key.includes(VERIFIED_BRO_HASH)) {
                return { name: 'BRO', appId: key };
            }
            return { name: 'Token', appId: key };
        }
    }
    return null;
}

/**
 * Extract beaming details from charm/spell data.
 * Handles multiple data nesting patterns from the API.
 */
function extractBeamingData(charm) {
    const nativeData = charm?.data?.native_data || charm?.spell?.native_data || charm?.data;
    if (!nativeData?.tx) return null;

    const tx = nativeData.tx;
    const beamedOuts = tx.beamed_outs;
    if (!beamedOuts) return null;

    // Parse outputs: each entry in `outs` is { "asset_index": amount }
    const outs = tx.outs || [];

    // Parse coins: BTC outputs with {amount, dest} for address info
    const coins = tx.coins || [];

    // Parse inputs
    const ins = tx.ins || [];

    // Detect which token is being beamed
    const token = detectBeamedToken(nativeData);

    // Build beaming entries
    const beamEntries = Object.entries(beamedOuts).map(([outIndex, destHash]) => {
        const idx = parseInt(outIndex);
        const outData = outs[idx];

        // Extract token amounts from the output (format: {"0": amount})
        let tokenAmounts = [];
        if (outData && typeof outData === 'object') {
            tokenAmounts = Object.entries(outData).map(([assetIdx, amount]) => ({
                assetIndex: assetIdx,
                amount
            }));
        }

        // Get the corresponding coin (BTC output) for address info
        const coin = coins[idx];

        return {
            outputIndex: idx,
            destinationHash: destHash,
            tokenAmounts,
            btcAmount: coin?.amount || null,
        };
    });

    // Calculate total tokens being beamed
    let totalTokens = 0;
    for (const entry of beamEntries) {
        for (const ta of entry.tokenAmounts) {
            totalTokens += ta.amount;
        }
    }

    return {
        beamEntries,
        inputs: ins,
        totalOutputs: outs.length,
        version: nativeData.version,
        token,
        totalTokens,
    };
}

export default function BeamingDetails({ charm, copyToClipboard }) {
    const beamingData = extractBeamingData(charm);
    if (!beamingData) return null;

    const { beamEntries, inputs, token, totalTokens } = beamingData;
    const tokenLabel = token?.name || 'tokens';

    return (
        <div className="bg-dark-800/50 rounded-lg p-4 border border-cyan-500/30">
            <h3 className="text-lg font-semibold text-white mb-2 flex items-center gap-2">
                <span>📡</span>
                <span>Beaming Details</span>
                <span className="text-xs px-2 py-0.5 rounded-full bg-cyan-500/20 text-cyan-400 border border-cyan-500/30 ml-2">
                    Beam Out
                </span>
            </h3>

            {/* Summary: what's being beamed */}
            <div className="mb-4 bg-dark-900/50 rounded-lg p-3 border border-dark-700/50">
                <p className="text-sm text-dark-300">
                    <span className="text-cyan-400 font-semibold">{formatTokenAmount(totalTokens)} {tokenLabel}</span>
                    {' '}locked to {beamEntries.length === 1 ? 'a commitment hash' : `${beamEntries.length} commitment hashes`}.
                    The recipient claims the tokens by revealing the matching preimage in an unbeam transaction.
                </p>
            </div>

            {/* Flow diagram */}
            <div className="mb-4 flex items-center gap-3 text-xs text-dark-400">
                <span className="bg-dark-900/50 rounded px-2 py-1 border border-dark-700/50">
                    Sender UTXO
                </span>
                <span className="text-cyan-500">→</span>
                <span className="bg-cyan-900/30 rounded px-2 py-1 border border-cyan-500/30 text-cyan-400">
                    Lock {formatTokenAmount(totalTokens)} {tokenLabel}
                </span>
                <span className="text-cyan-500">→</span>
                <span className="bg-dark-900/50 rounded px-2 py-1 border border-dark-700/50">
                    Hash commitment
                </span>
                <span className="text-dark-500">→</span>
                <span className="bg-dark-900/50 rounded px-2 py-1 border border-dark-700/50 opacity-50">
                    Unbeam (recipient)
                </span>
            </div>

            {/* Source Input */}
            {inputs.length > 0 && (
                <div className="mb-4">
                    <p className="text-xs text-dark-400 mb-2 uppercase tracking-wider">Source Input</p>
                    {inputs.map((input, idx) => (
                        <div key={idx} className="flex items-center gap-2 bg-dark-900/50 rounded-lg p-3 mb-1">
                            <span className="text-dark-500 text-xs">#{idx}</span>
                            <code className="text-xs text-primary-400 font-mono break-all flex-1">
                                {input}
                            </code>
                            {copyToClipboard && (
                                <button
                                    onClick={() => copyToClipboard(input)}
                                    className="flex-shrink-0 p-1 hover:bg-dark-700 rounded transition-colors"
                                    title="Copy UTXO"
                                >
                                    <svg className="w-3.5 h-3.5 text-dark-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                                    </svg>
                                </button>
                            )}
                        </div>
                    ))}
                </div>
            )}

            {/* Beamed Outputs */}
            <div>
                <p className="text-xs text-dark-400 mb-2 uppercase tracking-wider">
                    Beamed Outputs ({beamEntries.length})
                </p>
                <div className="space-y-3">
                    {beamEntries.map((entry, idx) => (
                        <div key={idx} className="bg-dark-900/50 rounded-lg p-4 border border-dark-700/50">
                            <div className="flex items-start justify-between mb-3">
                                <div className="flex items-center gap-2">
                                    <span className="text-dark-500 text-xs font-mono">vout:{entry.outputIndex}</span>
                                    <span className="text-cyan-400">→</span>
                                </div>
                                {/* Token amounts */}
                                <div className="text-right">
                                    {entry.tokenAmounts.map((ta, taIdx) => (
                                        <p key={taIdx} className="text-cyan-400 font-mono text-sm font-semibold">
                                            {formatTokenAmount(ta.amount)}
                                            <span className="text-dark-400 text-xs ml-1">{tokenLabel}</span>
                                        </p>
                                    ))}
                                    {entry.btcAmount && (
                                        <p className="text-orange-400 font-mono text-xs mt-0.5">
                                            {entry.btcAmount} sats
                                        </p>
                                    )}
                                </div>
                            </div>

                            {/* Destination commitment hash */}
                            <div>
                                <p className="text-xs text-dark-500 mb-1">Destination Commitment</p>
                                <div className="flex items-center gap-2">
                                    <code className="text-xs text-purple-400 font-mono break-all flex-1">
                                        {entry.destinationHash}
                                    </code>
                                    {copyToClipboard && (
                                        <button
                                            onClick={() => copyToClipboard(entry.destinationHash)}
                                            className="flex-shrink-0 p-1 hover:bg-dark-700 rounded transition-colors"
                                            title="Copy Destination Hash"
                                        >
                                            <svg className="w-3.5 h-3.5 text-dark-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                                            </svg>
                                        </button>
                                    )}
                                </div>
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
}
