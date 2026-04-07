'use client';

/**
 * Transaction Header Component
 * Displays transaction type icon, label, and status in a prominent header.
 * For beaming TXs, shows an inline chain flow (Bitcoin ↔ Cardano circles).
 */

import { getTransactionMetadata } from '@/services/transactions/transactionClassifier';

export default function TransactionHeader({ type, status = 'confirmed', amount, ticker, label, description, icon, beamFlow }) {
    const metadata = getTransactionMetadata(type);

    const getStatusColor = (status) => {
        switch (status) {
            case 'confirmed': return 'text-green-400 bg-green-500/20';
            case 'pending': return 'text-yellow-400 bg-yellow-500/20';
            case 'failed': return 'text-red-400 bg-red-500/20';
            default: return 'text-dark-400 bg-dark-500/20';
        }
    };

    // Beaming layout
    if (beamFlow) {
        return (
            <div className="flex items-start justify-between gap-4">
                <div className="flex-1">
                    {/* Title + Confirmed on same line */}
                    <div className="flex items-center gap-3 mb-3">
                        <h2 className="text-2xl font-bold text-white">
                            {label || metadata.label}
                        </h2>
                        <span className={`px-3 py-1 rounded-full text-sm font-medium capitalize ${getStatusColor(status)}`}>
                            {status}
                        </span>
                    </div>

                    {/* Chain flow below, aligned left */}
                    <div className="flex items-center gap-0">
                        <div className="flex items-center gap-1.5">
                            <div className={`w-8 h-8 rounded-full flex items-center justify-center border-2 ${
                                beamFlow.isBeamOut
                                    ? 'bg-orange-500/20 border-orange-500/50'
                                    : 'bg-blue-500/15 border-blue-500/30'
                            }`}>
                                <span className="text-xs font-bold">{beamFlow.isBeamOut ? '₿' : '₳'}</span>
                            </div>
                            <span className={`text-xs font-medium ${beamFlow.isBeamOut ? 'text-orange-400' : 'text-blue-400'}`}>
                                {beamFlow.isBeamOut ? 'Bitcoin' : 'Cardano'}
                            </span>
                        </div>
                        <div className="flex items-center mx-2">
                            <div className="w-8 h-[2px] bg-gradient-to-r from-cyan-500/50 to-cyan-400"></div>
                            <svg className="w-2.5 h-2.5 text-cyan-400 -ml-0.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                                <path d="M5 12h14m-4-4l4 4-4 4" />
                            </svg>
                        </div>
                        <div className="flex items-center gap-1.5">
                            <div className={`w-8 h-8 rounded-full flex items-center justify-center border-2 ${
                                beamFlow.isBeamOut
                                    ? 'bg-blue-500/15 border-blue-500/30'
                                    : 'bg-orange-500/20 border-orange-500/50'
                            }`}>
                                <span className="text-xs font-bold">{beamFlow.isBeamOut ? '₳' : '₿'}</span>
                            </div>
                            <span className={`text-xs font-medium ${beamFlow.isBeamOut ? 'text-blue-400' : 'text-orange-400'}`}>
                                {beamFlow.isBeamOut ? 'Cardano' : 'Bitcoin'}
                            </span>
                        </div>
                    </div>
                </div>

                {/* Description top-right, single line */}
                <p className="text-dark-400 text-sm whitespace-nowrap">
                    {description || metadata.description}
                </p>
            </div>
        );
    }

    // Default layout for non-beaming TXs
    return (
        <div className="flex items-start justify-between gap-4">
            <div className="flex items-start gap-4 flex-1">
                {/* Icon Circle */}
                <div className={`w-16 h-16 rounded-full flex items-center justify-center flex-shrink-0 ${metadata.bgClass} border-2 ${metadata.borderClass}`}>
                    <span className="text-3xl">{icon || metadata.icon}</span>
                </div>

                {/* Title and Status */}
                <div className="flex-1">
                    <h2 className="text-2xl font-bold text-white mb-2">
                        {label || metadata.label}
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
                    <p className="text-dark-400 text-sm mt-2">{description || metadata.description}</p>
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
