'use client';

/**
 * Transaction Badge Component
 * Displays a styled badge for transaction type
 */

import { getTransactionMetadata } from '@/services/transactions/transactionClassifier';

export default function TransactionBadge({ type, size = 'md', showIcon = true }) {
    const metadata = getTransactionMetadata(type);
    
    const sizeClasses = {
        sm: 'px-2 py-0.5 text-xs',
        md: 'px-3 py-1 text-sm',
        lg: 'px-4 py-1.5 text-base'
    };
    
    return (
        <span className={`inline-flex items-center gap-1.5 rounded-full font-medium ${metadata.bgClass} ${metadata.textClass} border ${metadata.borderClass} ${sizeClasses[size]}`}>
            {showIcon && <span>{metadata.icon}</span>}
            <span>{metadata.label}</span>
        </span>
    );
}
