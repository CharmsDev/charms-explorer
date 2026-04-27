'use client';

/**
 * Beaming Details Component
 * Displays detailed information about Beaming transactions (cross-chain token transfers).
 *
 * Beaming transfers tokens between Bitcoin and Cardano without bridges or custodians:
 * - Beam Out (Bitcoin → Cardano): tokens locked to a commitment hash on Bitcoin,
 *   then minted as proxy CNTs on Cardano once Bitcoin finality is proven.
 * - Beam In (Cardano → Bitcoin): proxy CNTs burned on Cardano, original tokens
 *   recreated on Bitcoin once Cardano finality is proven.
 *
 * Props:
 *   charm          - The charm/transaction object with spell data
 *   copyToClipboard - Function to copy text
 *   beamDirection  - "beam_out" or "beam_in" (from classifier)
 *   assets         - Array of asset records (may contain cardano_policy_id, cardano_fingerprint)
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
 * Detect which token is involved from app_public_inputs keys.
 */
function detectBeamedToken(nativeData) {
    const appInputs = nativeData?.app_public_inputs;
    if (!appInputs) return null;
    for (const key of Object.keys(appInputs)) {
        if (key.startsWith('t/')) {
            if (key.includes(VERIFIED_BRO_HASH)) return { name: 'BRO', appId: key };
            return { name: 'Token', appId: key };
        }
    }
    return null;
}

/**
 * Extract beaming data from spell for BEAM OUT (Bitcoin → Cardano).
 * The spell has `beamed_outs` mapping output indices to destination commitment hashes.
 */
function extractBeamOutData(charm) {
    const nativeData = charm?.data?.native_data || charm?.spell?.native_data || charm?.data;
    if (!nativeData?.tx) return null;

    const tx = nativeData.tx;
    const beamedOuts = tx.beamed_outs;
    if (!beamedOuts) return null;

    const outs = tx.outs || [];
    const coins = tx.coins || [];
    const ins = tx.ins || [];
    const token = detectBeamedToken(nativeData);

    const beamEntries = Object.entries(beamedOuts).map(([outIndex, destHash]) => {
        const idx = parseInt(outIndex);
        const outData = outs[idx];
        let tokenAmounts = [];
        if (outData && typeof outData === 'object') {
            tokenAmounts = Object.entries(outData).map(([assetIdx, amount]) => ({
                assetIndex: assetIdx,
                amount
            }));
        }
        const coin = coins[idx];
        return {
            outputIndex: idx,
            destinationHash: destHash,
            tokenAmounts,
            btcAmount: coin?.amount || null,
        };
    });

    let totalTokens = 0;
    for (const entry of beamEntries) {
        for (const ta of entry.tokenAmounts) totalTokens += ta.amount;
    }

    return { beamEntries, inputs: ins, token, totalTokens };
}

/**
 * Extract beaming data from spell for BEAM IN (Cardano → Bitcoin).
 * The spell has a `c/` contract app in app_public_inputs (the bridge contract).
 * Outputs contain the recreated tokens on Bitcoin.
 */
function extractBeamInData(charm) {
    const nativeData = charm?.data?.native_data || charm?.spell?.native_data || charm?.data;
    if (!nativeData?.tx) return null;

    const tx = nativeData.tx;
    const appInputs = nativeData.app_public_inputs;
    if (!appInputs) return null;

    const contractKey = Object.keys(appInputs).find(k => k.startsWith('c/'));
    if (!contractKey) return null;

    const token = detectBeamedToken(nativeData);
    const outs = tx.outs || [];
    const ins = tx.ins || [];

    let totalTokens = 0;
    const outputEntries = [];
    outs.forEach((out, idx) => {
        if (!out || typeof out !== 'object') return;
        const tokenAmounts = Object.entries(out).map(([assetIdx, amount]) => ({
            assetIndex: assetIdx,
            amount
        }));
        if (tokenAmounts.length > 0) {
            for (const ta of tokenAmounts) totalTokens += ta.amount;
            outputEntries.push({ outputIndex: idx, tokenAmounts });
        }
    });

    return { outputEntries, contractKey, token, totalTokens, inputs: ins };
}

function CopyButton({ text, copyToClipboard, title }) {
    if (!copyToClipboard) return null;
    return (
        <button
            onClick={() => copyToClipboard(text)}
            className="flex-shrink-0 p-1 hover:bg-dark-700 rounded transition-colors"
            title={title}
        >
            <svg className="w-3.5 h-3.5 text-dark-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
            </svg>
        </button>
    );
}

export default function BeamingDetails({ charm, copyToClipboard, beamDirection, assets }) {
    const isBeamOut = beamDirection !== 'beam_in';
    const beamOutData = isBeamOut ? extractBeamOutData(charm) : null;
    const beamInData = !isBeamOut ? extractBeamInData(charm) : null;

    if (!beamOutData && !beamInData) return null;

    const token = (beamOutData || beamInData).token;
    const totalTokens = (beamOutData || beamInData).totalTokens;
    const inputs = (beamOutData || beamInData).inputs;
    const tokenLabel = token?.name || 'tokens';

    // Extract Cardano metadata from assets
    const cardanoAssets = (assets || []).filter(a => a.cardano_fingerprint || a.cardano_policy_id);

    return (
        <div className="bg-dark-800/50 rounded-lg p-4 border border-cyan-500/30">
            {/* Header */}
            <h3 className="text-lg font-semibold text-white mb-2 flex items-center gap-2 flex-wrap">
                <span>{isBeamOut ? '📤' : '📥'}</span>
                <span>{isBeamOut ? 'Beam Out' : 'Mint / Beam In'}</span>
                <span className="text-xs px-2 py-0.5 rounded-full bg-cyan-500/20 text-cyan-400 border border-cyan-500/30">
                    {isBeamOut ? 'Bitcoin → Cardano' : 'Cardano → Bitcoin'}
                </span>
            </h3>

            {/* Summary */}
            <div className="mb-4 bg-dark-900/50 rounded-lg p-3 border border-dark-700/50">
                <p className="text-sm text-dark-300">
                    <span className="text-cyan-400 font-semibold">{formatTokenAmount(totalTokens)} {tokenLabel}</span>
                    {isBeamOut
                        ? <> beamed to Cardano via {beamOutData.beamEntries.length === 1 ? 'a commitment hash' : `${beamOutData.beamEntries.length} commitment hashes`}. Tokens will be minted as proxy CNTs on Cardano once Bitcoin finality is proven.</>
                        : <> received from Cardano to Bitcoin. Proxy CNTs were burned on Cardano and original tokens recreated on Bitcoin.</>
                    }
                </p>
            </div>

            {/* Flow diagram */}
            <div className="mb-4 flex items-center gap-2 text-xs text-dark-400 flex-wrap">
                {isBeamOut ? (
                    <>
                        <span className="bg-orange-900/30 rounded px-2 py-1 border border-orange-500/30 text-orange-400">Bitcoin</span>
                        <span className="text-cyan-500">→</span>
                        <span className="bg-cyan-900/30 rounded px-2 py-1 border border-cyan-500/30 text-cyan-400">
                            Lock {formatTokenAmount(totalTokens)} {tokenLabel}
                        </span>
                        <span className="text-cyan-500">→</span>
                        <span className="bg-dark-900/50 rounded px-2 py-1 border border-dark-700/50">Commitment hash</span>
                        <span className="text-cyan-500">→</span>
                        <span className="bg-blue-900/30 rounded px-2 py-1 border border-blue-500/30 text-blue-400">Cardano (mint CNTs)</span>
                    </>
                ) : (
                    <>
                        <span className="bg-blue-900/30 rounded px-2 py-1 border border-blue-500/30 text-blue-400">Cardano (burn CNTs)</span>
                        <span className="text-cyan-500">→</span>
                        <span className="bg-dark-900/50 rounded px-2 py-1 border border-dark-700/50">Finality proof</span>
                        <span className="text-cyan-500">→</span>
                        <span className="bg-cyan-900/30 rounded px-2 py-1 border border-cyan-500/30 text-cyan-400">
                            Recreate {formatTokenAmount(totalTokens)} {tokenLabel}
                        </span>
                        <span className="text-cyan-500">→</span>
                        <span className="bg-orange-900/30 rounded px-2 py-1 border border-orange-500/30 text-orange-400">Bitcoin</span>
                    </>
                )}
            </div>

            {/* Cardano Asset Info (if available) */}
            {cardanoAssets.length > 0 && (
                <div className="mb-4">
                    <p className="text-xs text-dark-400 mb-2 uppercase tracking-wider">Cardano Asset</p>
                    {cardanoAssets.map((asset, idx) => (
                        <div key={idx} className="bg-dark-900/50 rounded-lg p-3 border border-dark-700/50 space-y-1.5">
                            {asset.cardano_fingerprint && (
                                <div className="flex items-center justify-between gap-2">
                                    <span className="text-xs text-dark-500 shrink-0">Fingerprint</span>
                                    <div className="flex items-center gap-2">
                                        <code className="text-xs text-cyan-400 font-mono">{asset.cardano_fingerprint}</code>
                                        <a
                                            href={`https://cardanoscan.io/token/${asset.cardano_fingerprint}`}
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            className="text-xs text-primary-400 hover:text-primary-300 shrink-0"
                                        >
                                            Cardanoscan →
                                        </a>
                                    </div>
                                </div>
                            )}
                            {asset.cardano_policy_id && (
                                <div className="flex items-center justify-between gap-2">
                                    <span className="text-xs text-dark-500 shrink-0">Policy ID</span>
                                    <code className="text-xs text-purple-400 font-mono break-all">{asset.cardano_policy_id}</code>
                                </div>
                            )}
                        </div>
                    ))}
                </div>
            )}

            {/* Source Inputs */}
            {inputs.length > 0 && (
                <div className="mb-4">
                    <p className="text-xs text-dark-400 mb-2 uppercase tracking-wider">Source Inputs</p>
                    {inputs.map((input, idx) => (
                        <div key={idx} className="flex items-center gap-2 bg-dark-900/50 rounded-lg p-3 mb-1">
                            <span className="text-dark-500 text-xs">#{idx}</span>
                            <code className="text-xs text-primary-400 font-mono break-all flex-1">{input}</code>
                            <CopyButton text={input} copyToClipboard={copyToClipboard} title="Copy UTXO" />
                        </div>
                    ))}
                </div>
            )}

            {/* Beam Out: show commitment hashes */}
            {isBeamOut && beamOutData && (
                <div>
                    <p className="text-xs text-dark-400 mb-2 uppercase tracking-wider">
                        Beamed Outputs ({beamOutData.beamEntries.length})
                    </p>
                    <div className="space-y-3">
                        {beamOutData.beamEntries.map((entry, idx) => (
                            <div key={idx} className="bg-dark-900/50 rounded-lg p-4 border border-dark-700/50">
                                <div className="flex items-start justify-between mb-3">
                                    <div className="flex items-center gap-2">
                                        <span className="text-dark-500 text-xs font-mono">vout:{entry.outputIndex}</span>
                                        <span className="text-cyan-400">→</span>
                                    </div>
                                    <div className="text-right">
                                        {entry.tokenAmounts.map((ta, taIdx) => (
                                            <p key={taIdx} className="text-cyan-400 font-mono text-sm font-semibold">
                                                {formatTokenAmount(ta.amount)}
                                                <span className="text-dark-400 text-xs ml-1">{tokenLabel}</span>
                                            </p>
                                        ))}
                                        {entry.btcAmount && (
                                            <p className="text-orange-400 font-mono text-xs mt-0.5">{entry.btcAmount} sats</p>
                                        )}
                                    </div>
                                </div>
                                <div>
                                    <p className="text-xs text-dark-500 mb-1">Destination Commitment (Cardano placeholder UTXO hash)</p>
                                    <div className="flex items-center gap-2">
                                        <code className="text-xs text-purple-400 font-mono break-all flex-1">{entry.destinationHash}</code>
                                        <CopyButton text={entry.destinationHash} copyToClipboard={copyToClipboard} title="Copy commitment hash" />
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            {/* Beam In: show recreated outputs */}
            {!isBeamOut && beamInData && (
                <div>
                    <p className="text-xs text-dark-400 mb-2 uppercase tracking-wider">
                        Recreated Outputs ({beamInData.outputEntries.length})
                    </p>
                    <div className="space-y-3">
                        {beamInData.outputEntries.map((entry, idx) => (
                            <div key={idx} className="bg-dark-900/50 rounded-lg p-3 border border-dark-700/50">
                                <div className="flex items-center justify-between">
                                    <div className="flex items-center gap-2">
                                        <span className="text-dark-500 text-xs font-mono">vout:{entry.outputIndex}</span>
                                        <span className="text-cyan-400">→</span>
                                    </div>
                                    <div className="text-right">
                                        {entry.tokenAmounts.map((ta, taIdx) => (
                                            <p key={taIdx} className="text-cyan-400 font-mono text-sm font-semibold">
                                                {formatTokenAmount(ta.amount)}
                                                <span className="text-dark-400 text-xs ml-1">{tokenLabel}</span>
                                            </p>
                                        ))}
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
}
