'use client';

/**
 * Beaming Details Component
 * Displays detailed information about Beaming transactions (cross-address token transfers).
 * 
 * Beaming txs transfer tokens from one address to another via the `beamed_outs` field
 * in the spell data. Each beamed_out maps an output index to a destination commitment hash.
 *
 * [RJJ-BEAMING] This component handles the "beam" direction (sending tokens).
 * TODO: [RJJ-UNBEAM] In the future, "unbeam" transactions will reverse this process.
 *       Unbeam txs will likely have a different spell structure (e.g., `unbeamed_ins`).
 *       When implementing unbeam, consider:
 *       - Adding an UnbeamDetails component or extending this one with a `direction` prop
 *       - Showing the source commitment hash being redeemed
 *       - Displaying the receiving address and claimed amount
 */

const TOKEN_DECIMALS = 8;

const formatTokenAmount = (rawAmount) => {
    if (rawAmount === undefined || rawAmount === null) return '-';
    const displayValue = rawAmount / Math.pow(10, TOKEN_DECIMALS);
    return displayValue.toLocaleString(undefined, {
        minimumFractionDigits: 0,
        maximumFractionDigits: 4
    });
};

const shortenHash = (hash, chars = 8) => {
    if (!hash || hash.length <= chars * 2 + 3) return hash || '-';
    return `${hash.substring(0, chars)}...${hash.substring(hash.length - chars)}`;
};

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

    return {
        beamEntries,
        inputs: ins,
        totalOutputs: outs.length,
        version: nativeData.version
    };
}

export default function BeamingDetails({ charm, copyToClipboard }) {
    const beamingData = extractBeamingData(charm);
    if (!beamingData) return null;

    const { beamEntries, inputs } = beamingData;

    return (
        <div className="bg-dark-800/50 rounded-lg p-4 border border-cyan-500/30">
            <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
                <span>📡</span>
                <span>Beaming Details</span>
                {/* TODO: [RJJ-UNBEAM] Show direction indicator: "Beam Out" vs "Unbeam In" */}
                <span className="text-xs px-2 py-0.5 rounded-full bg-cyan-500/20 text-cyan-400 border border-cyan-500/30 ml-2">
                    Beam Out
                </span>
            </h3>

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
                                            <span className="text-dark-400 text-xs ml-1">tokens</span>
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

            {/* Info note */}
            <div className="mt-4 pt-3 border-t border-dark-700/50">
                <p className="text-xs text-dark-500 italic">
                    Beaming transfers tokens to a destination commitment hash. The recipient 
                    can claim the tokens by providing the matching preimage in an unbeam transaction.
                </p>
                {/* TODO: [RJJ-UNBEAM] Add link to the corresponding unbeam tx if known */}
            </div>
        </div>
    );
}
