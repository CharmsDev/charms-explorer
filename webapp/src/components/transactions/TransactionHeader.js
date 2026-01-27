'use client';

/**
 * Transaction Header Component
 * Displays transaction type icon, label, and status in a prominent header
 */

import { getTransactionMetadata } from '@/services/transactions/transactionClassifier';

export default function TransactionHeader({ type, status = 'confirmed', amount, ticker }) {
    const metadata = getTransactionMetadata(type);
    
    const getStatusColor = (status) => {
        switch (status) {
            case 'confirmed': return 'text-green-400 bg-green-500/20';
            case 'pending': return 'text-yellow-400 bg-yellow-500/20';
            case 'failed': return 'text-red-400 bg-red-500/20';
            default: return 'text-dark-400 bg-dark-500/20';
        }
    };
    
    return (
        <div className="flex items-start justify-between gap-4">
            <div className="flex items-start gap-4 flex-1">
                {/* Icon Circle */}
                <div className={`w-16 h-16 rounded-full flex items-center justify-center flex-shrink-0 ${metadata.bgClass} border-2 ${metadata.borderClass}`}>
                    <span className="text-3xl">{metadata.icon}</span>
                </div>
                
                {/* Title and Status */}
                <div className="flex-1">
                    <h2 className="text-2xl font-bold text-white mb-2">
                        {metadata.label}
                    </h2>
                    <div className="flex items-center gap-2 flex-wrap">
                        <span className={`px-3 py-1 rounded-full text-sm font-medium capitalize ${getStatusColor(status)}`}>
                            {status}
                        </span>
                        {ticker && (
                            <span className={`px-3 py-1 rounded-full text-sm font-medium ${metadata.bgClass} ${metadata.textClass}`}>
                                {ticker}
                            </span>
                        )}
                    </div>
                    <p className="text-dark-400 text-sm mt-2">{metadata.description}</p>
                </div>
            </div>
            
            {/* Amount (if provided) */}
            {amount !== undefined && amount !== null && (
                <div className="text-right">
                    <p className={`text-2xl font-bold ${metadata.textClass}`}>
                        {typeof amount === 'number' ? amount.toLocaleString() : amount}
                    </p>
                    {ticker && (
                        <p className="text-sm text-dark-400 mt-1">{ticker}</p>
                    )}
                </div>
            )}
        </div>
    );
}
