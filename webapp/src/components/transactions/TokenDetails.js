'use client';

/**
 * Token Details Component
 * Displays token information including image, name, ticker, and app ID
 */

export default function TokenDetails({ 
    tokenName, 
    tokenTicker, 
    tokenImage, 
    appId, 
    amount,
    decimals = 9,
    copyToClipboard 
}) {
    const formatAmount = (rawAmount) => {
        if (!rawAmount) return '-';
        const displayValue = rawAmount / Math.pow(10, decimals);
        return displayValue.toLocaleString(undefined, { 
            minimumFractionDigits: 0, 
            maximumFractionDigits: 4 
        });
    };

    return (
        <div className="bg-dark-800/50 rounded-lg p-4 border border-dark-700">
            <div className="flex gap-4 items-start">
                {/* Token Image */}
                {tokenImage && (
                    <div className="w-20 flex-shrink-0">
                        <img 
                            src={tokenImage} 
                            alt={tokenName || 'Token'}
                            className="w-full h-auto rounded-lg object-cover border-2 border-purple-500/30"
                            onError={(e) => { e.target.style.display = 'none'; }}
                        />
                    </div>
                )}

                <div className={tokenImage ? "flex-1" : "w-full"}>
                    <p className="text-xs text-dark-400 mb-2">Token</p>
                    
                    {/* Name and Ticker */}
                    <div className="flex items-baseline gap-2 mb-3">
                        {tokenName && (
                            <h4 className="text-lg font-semibold text-white">{tokenName}</h4>
                        )}
                        {tokenTicker && (
                            <p className="text-sm text-purple-400 font-mono">{tokenTicker}</p>
                        )}
                    </div>

                    {/* Amount */}
                    {amount !== undefined && (
                        <div className="mb-3">
                            <p className="text-xs text-dark-400 mb-0.5">Amount</p>
                            <p className="text-lg text-white font-mono">
                                {formatAmount(amount)}
                                {tokenTicker && <span className="text-dark-400 text-sm ml-1">{tokenTicker}</span>}
                            </p>
                        </div>
                    )}

                    {/* App ID */}
                    {appId && (
                        <div>
                            <p className="text-xs text-dark-400 mb-1">App ID</p>
                            <div className="flex items-start gap-2 bg-dark-900/50 p-2 rounded">
                                <code className="text-xs text-purple-400 break-all font-mono flex-1 leading-tight">
                                    {appId}
                                </code>
                                {copyToClipboard && (
                                    <button
                                        onClick={() => copyToClipboard(appId)}
                                        className="flex-shrink-0 p-1 hover:bg-dark-700 rounded transition-colors"
                                        title="Copy App ID"
                                    >
                                        <svg className="w-3 h-3 text-dark-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                                        </svg>
                                    </button>
                                )}
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
