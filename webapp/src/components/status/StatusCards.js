'use client';

import { Badge } from '@/components/ui/Badge';

export default function StatusCards({ indexerStatus, bitcoinStatus, getStatusBadgeClass, getConnectionStatusBadgeClass, lastUpdated, networkType = 'testnet4' }) {
    // Define color schemes based on network type
    const colorScheme = networkType === 'mainnet'
        ? {
            accent: 'from-orange-400 to-red-500',
            highlight: 'text-orange-400'
        }
        : {
            accent: 'from-blue-400 to-indigo-500',
            highlight: 'text-blue-400'
        };

    return (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
            {/* Indexer Status Card */}
            <div className="bg-gradient-to-br from-dark-800 to-dark-900 rounded-lg overflow-hidden shadow-lg">
                <div className="p-6">
                    <div className="flex justify-between items-center mb-4">
                        <h2 className="text-xl font-semibold text-white">Indexer Status</h2>
                        <Badge className={`${getStatusBadgeClass(indexerStatus.status)} px-3 py-1`}>
                            {indexerStatus.status || 'Unknown'}
                        </Badge>
                    </div>
                    <div className="space-y-4">
                        <div className="bg-dark-800/50 p-3 rounded-lg">
                            <div className="text-sm text-dark-400 mb-1">Network</div>
                            <div className={`text-lg font-semibold ${colorScheme.highlight}`}>
                                {networkType === 'mainnet' ? 'Bitcoin Mainnet' : 'Bitcoin Testnet 4'}
                            </div>
                        </div>
                        <div className="bg-dark-800/50 p-3 rounded-lg">
                            <div className="text-sm text-dark-400 mb-1">Last Processed Block</div>
                            <div className="text-lg font-semibold text-white">{indexerStatus.last_processed_block || '0'}</div>
                        </div>
                        <div className="bg-dark-800/50 p-3 rounded-lg">
                            <div className="text-sm text-dark-400 mb-1">Latest Confirmed Block</div>
                            <div className="text-lg font-semibold text-white">{indexerStatus.latest_confirmed_block || '0'}</div>
                        </div>
                        <div className="bg-dark-800/50 p-3 rounded-lg">
                            <div className="text-sm text-dark-400 mb-1">Last Updated</div>
                            <div className="text-lg font-semibold text-white">{lastUpdated.toLocaleString()}</div>
                        </div>
                    </div>
                </div>
            </div>

            {/* Bitcoin Node Card */}
            <div className="bg-gradient-to-br from-dark-800 to-dark-900 rounded-lg overflow-hidden shadow-lg">
                <div className="p-6">
                    <div className="flex justify-between items-center mb-4">
                        <h2 className="text-xl font-semibold text-white">Bitcoin Node</h2>
                        <Badge className={`${getConnectionStatusBadgeClass(bitcoinStatus.status)} px-3 py-1`}>
                            {bitcoinStatus.status || 'Unknown'}
                        </Badge>
                    </div>
                    <div className="space-y-4">
                        <div className="bg-dark-800/50 p-3 rounded-lg">
                            <div className="text-sm text-dark-400 mb-1">Network</div>
                            <div className={`text-lg font-semibold ${colorScheme.highlight}`}>
                                {networkType === 'mainnet' ? 'Bitcoin Mainnet' : 'Bitcoin Testnet 4'}
                            </div>
                        </div>
                        <div className="bg-dark-800/50 p-3 rounded-lg">
                            <div className="text-sm text-dark-400 mb-1">Current Block Height</div>
                            <div className="text-lg font-semibold text-white">{bitcoinStatus.block_count || '0'}</div>
                        </div>
                        <div className="bg-dark-800/50 p-3 rounded-lg">
                            <div className="text-sm text-dark-400 mb-1">Best Block Hash</div>
                            <div className="text-lg font-semibold text-white truncate" title={bitcoinStatus.best_block_hash || '-'}>
                                {bitcoinStatus.best_block_hash ?
                                    (bitcoinStatus.best_block_hash.length > 20 ?
                                        `${bitcoinStatus.best_block_hash.substring(0, 10)}...${bitcoinStatus.best_block_hash.substring(bitcoinStatus.best_block_hash.length - 10)}`
                                        : bitcoinStatus.best_block_hash)
                                    : '-'}
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
