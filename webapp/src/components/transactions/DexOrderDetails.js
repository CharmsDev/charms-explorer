'use client';

/**
 * DEX Order Details Component
 * Displays detailed information about DEX orders (Ask, Bid, Fulfill, Cancel)
 */

const TOKEN_DECIMALS = 9;

const formatTokenQuantity = (rawQuantity) => {
    if (!rawQuantity) return '-';
    const displayValue = rawQuantity / Math.pow(10, TOKEN_DECIMALS);
    return displayValue.toLocaleString(undefined, { 
        minimumFractionDigits: 0, 
        maximumFractionDigits: 2 
    });
};

const formatPrice = (price) => {
    if (!price || !Array.isArray(price)) return '-';
    const [numerator, denominator] = price;
    if (!denominator) return '-';
    const pricePerToken = numerator / denominator;
    return `${pricePerToken.toLocaleString()} sats/token`;
};

export default function DexOrderDetails({ orderDetails, copyToClipboard }) {
    if (!orderDetails) return null;
    
    const { side, amount, quantity, price, maker, asset } = orderDetails;
    
    return (
        <div className="bg-dark-800/50 rounded-lg p-4 border border-dark-700">
            <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
                <span>{side === 'ask' ? '📈' : '📉'}</span>
                <span>{side === 'ask' ? 'Ask Order' : 'Bid Order'} Details</span>
            </h3>
            
            <div className="grid grid-cols-2 gap-4">
                {/* Quantity */}
                <div>
                    <p className="text-xs text-dark-400 mb-1">Quantity</p>
                    <p className="text-white font-mono text-lg">
                        {formatTokenQuantity(quantity)}
                        <span className="text-dark-400 text-sm ml-1">tokens</span>
                    </p>
                </div>
                
                {/* Amount */}
                <div>
                    <p className="text-xs text-dark-400 mb-1">Amount</p>
                    <p className="text-orange-400 font-mono text-lg">
                        {amount?.toLocaleString() || '-'}
                        <span className="text-dark-400 text-sm ml-1">sats</span>
                    </p>
                </div>
                
                {/* Price */}
                {price && (
                    <div>
                        <p className="text-xs text-dark-400 mb-1">Price</p>
                        <p className="text-white font-mono">
                            {formatPrice(price)}
                        </p>
                    </div>
                )}
                
                {/* Side */}
                <div>
                    <p className="text-xs text-dark-400 mb-1">Side</p>
                    <span className={`inline-flex items-center px-2 py-1 rounded text-xs font-medium ${
                        side === 'ask' 
                            ? 'bg-green-500/20 text-green-400' 
                            : 'bg-blue-500/20 text-blue-400'
                    }`}>
                        {side === 'ask' ? '📈 Sell (Ask)' : '📉 Buy (Bid)'}
                    </span>
                </div>
            </div>
            
            {/* Maker Address */}
            {maker && (
                <div className="mt-4 pt-4 border-t border-dark-700">
                    <p className="text-xs text-dark-400 mb-1">Maker Address</p>
                    <div className="flex items-center gap-2">
                        <code className="text-xs text-primary-400 font-mono break-all flex-1">
                            {maker}
                        </code>
                        {copyToClipboard && (
                            <button
                                onClick={() => copyToClipboard(maker)}
                                className="flex-shrink-0 p-1 hover:bg-dark-700 rounded transition-colors"
                                title="Copy Address"
                            >
                                <svg className="w-4 h-4 text-dark-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                                </svg>
                            </button>
                        )}
                    </div>
                </div>
            )}
            
            {/* Asset */}
            {asset && (
                <div className="mt-4 pt-4 border-t border-dark-700">
                    <p className="text-xs text-dark-400 mb-1">Asset</p>
                    <code className="text-xs text-purple-400 font-mono break-all">
                        {asset}
                    </code>
                </div>
            )}
        </div>
    );
}
