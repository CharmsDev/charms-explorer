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

    // Beaming layout: title left, SVG flow right
    if (beamFlow) {
        const isBO = beamFlow.isBeamOut;
        return (
            <div className="flex items-center justify-between gap-6">
                {/* Left: title + confirmed + description */}
                <div className="flex-shrink-0">
                    <div className="flex items-center gap-3 mb-1">
                        <h2 className="text-2xl font-bold text-white">
                            {label || metadata.label}
                        </h2>
                        <span className={`px-3 py-1 rounded-full text-sm font-medium capitalize ${getStatusColor(status)}`}>
                            {status}
                        </span>
                    </div>
                    <p className="text-dark-400 text-sm">
                        {description || metadata.description}
                    </p>
                </div>

                {/* Right: SVG beam flow diagram */}
                <div className="flex-shrink-0">
                    <svg width="280" height="72" viewBox="0 0 280 72" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <defs>
                            <linearGradient id="beamGrad" x1="0%" y1="0%" x2="100%" y2="0%">
                                <stop offset="0%" stopColor="#06b6d4" stopOpacity="0.3" />
                                <stop offset="100%" stopColor="#06b6d4" stopOpacity="0.8" />
                            </linearGradient>
                            <filter id="glowBtc"><feDropShadow dx="0" dy="0" stdDeviation="3" floodColor="#f97316" floodOpacity="0.4" /></filter>
                            <filter id="glowAda"><feDropShadow dx="0" dy="0" stdDeviation="3" floodColor="#3b82f6" floodOpacity="0.4" /></filter>
                            <filter id="glowPlaceholder"><feDropShadow dx="0" dy="0" stdDeviation="2" floodColor="#3b82f6" floodOpacity="0.2" /></filter>
                        </defs>

                        {isBO ? (
                            <>
                                {/* Beam Out: Placeholder(small) -> Bitcoin(main) -> Cardano */}

                                {/* Curved line from placeholder to Cardano destination (above) */}
                                <path d="M 30 26 Q 140 -10 250 26" stroke="#3b82f6" strokeWidth="1" strokeDasharray="4 3" fill="none" opacity="0.35" />
                                <text x="140" y="8" textAnchor="middle" fill="#64748b" fontSize="8" fontFamily="monospace">placeholder link</text>

                                {/* Placeholder circle (small, muted, left) */}
                                <circle cx="30" cy="36" r="14" fill="rgba(59,130,246,0.08)" stroke="#3b82f6" strokeWidth="1.5" strokeDasharray="3 2" filter="url(#glowPlaceholder)" />
                                <text x="30" y="40" textAnchor="middle" fill="#3b82f6" fontSize="10" fontWeight="600" fontFamily="system-ui">₳</text>
                                <text x="30" y="58" textAnchor="middle" fill="#475569" fontSize="8" fontFamily="system-ui">placeholder</text>

                                {/* Arrow: placeholder -> Bitcoin (dashed) */}
                                <line x1="46" y1="36" x2="96" y2="36" stroke="#64748b" strokeWidth="1" strokeDasharray="3 2" />
                                <polygon points="96,33 102,36 96,39" fill="#64748b" />

                                {/* Bitcoin circle (main, prominent) */}
                                <circle cx="120" cy="36" r="20" fill="rgba(249,115,22,0.15)" stroke="#f97316" strokeWidth="2" filter="url(#glowBtc)" />
                                <text x="120" y="41" textAnchor="middle" fill="#f97316" fontSize="14" fontWeight="700" fontFamily="system-ui">₿</text>
                                <text x="120" y="66" textAnchor="middle" fill="#f97316" fontSize="9" fontWeight="600" fontFamily="system-ui">Bitcoin</text>

                                {/* Arrow: Bitcoin -> Cardano (solid gradient) */}
                                <line x1="142" y1="36" x2="226" y2="36" stroke="url(#beamGrad)" strokeWidth="2" />
                                <polygon points="226,32 234,36 226,40" fill="#06b6d4" />

                                {/* Cardano circle (destination) */}
                                <circle cx="250" cy="36" r="18" fill="rgba(59,130,246,0.1)" stroke="#3b82f6" strokeWidth="2" filter="url(#glowAda)" />
                                <text x="250" y="41" textAnchor="middle" fill="#3b82f6" fontSize="13" fontWeight="700" fontFamily="system-ui">₳</text>
                                <text x="250" y="66" textAnchor="middle" fill="#3b82f6" fontSize="9" fontWeight="600" fontFamily="system-ui">Cardano</text>
                            </>
                        ) : (
                            <>
                                {/* Beam In: Cardano(source) -> Bitcoin(main) */}

                                {/* Cardano circle (source) */}
                                <circle cx="60" cy="36" r="18" fill="rgba(59,130,246,0.1)" stroke="#3b82f6" strokeWidth="2" filter="url(#glowAda)" />
                                <text x="60" y="41" textAnchor="middle" fill="#3b82f6" fontSize="13" fontWeight="700" fontFamily="system-ui">₳</text>
                                <text x="60" y="66" textAnchor="middle" fill="#3b82f6" fontSize="9" fontWeight="600" fontFamily="system-ui">Cardano</text>

                                {/* Arrow: Cardano -> Bitcoin */}
                                <line x1="80" y1="36" x2="190" y2="36" stroke="url(#beamGrad)" strokeWidth="2" />
                                <polygon points="190,32 198,36 190,40" fill="#06b6d4" />

                                {/* Bitcoin circle (destination, main) */}
                                <circle cx="216" cy="36" r="20" fill="rgba(249,115,22,0.15)" stroke="#f97316" strokeWidth="2" filter="url(#glowBtc)" />
                                <text x="216" y="41" textAnchor="middle" fill="#f97316" fontSize="14" fontWeight="700" fontFamily="system-ui">₿</text>
                                <text x="216" y="66" textAnchor="middle" fill="#f97316" fontSize="9" fontWeight="600" fontFamily="system-ui">Bitcoin</text>
                            </>
                        )}
                    </svg>
                </div>
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
