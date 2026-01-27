'use client';

import { Table, TableHeader, TableBody, TableRow, TableCell } from '@/components/ui/Table';

export default function CharmStatistics({ charmStats, tagStats = {}, networkType = 'testnet4' }) {
    // Define color schemes based on network type
    const colorScheme = networkType === 'mainnet'
        ? {
            gradient: 'from-orange-400 to-red-600',
            hover: 'hover:bg-dark-800/70'
        }
        : {
            gradient: 'from-primary-400 to-primary-600',
            hover: 'hover:bg-dark-800/70'
        };

    // Format large numbers with commas
    const formatNumber = (num) => {
        return (num || 0).toLocaleString();
    };

    return (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
            {/* Charm Statistics Card */}
            <div className="bg-gradient-to-br from-dark-800 to-dark-900 rounded-lg overflow-hidden shadow-lg">
                <div className="p-6">
                    <h2 className="text-xl font-semibold text-white mb-4">
                        Charm Statistics
                        <span className="ml-2 text-sm font-normal text-dark-400">
                            {networkType === 'mainnet' ? '(Mainnet)' : '(Testnet 4)'}
                        </span>
                    </h2>
                    <div className="grid grid-cols-2 gap-4">
                        <div className={`bg-dark-800/50 p-4 rounded-lg ${colorScheme.hover} transition-colors`}>
                            <div className="text-sm text-gray-400 mb-1">Total Charms</div>
                            <div className={`text-2xl font-bold bg-gradient-to-r ${colorScheme.gradient} bg-clip-text text-transparent`}>
                                {charmStats.total_charms || '0'}
                            </div>
                        </div>
                        <div className={`bg-dark-800/50 p-4 rounded-lg ${colorScheme.hover} transition-colors`}>
                            <div className="text-sm text-gray-400 mb-1">Total Transactions</div>
                            <div className={`text-2xl font-bold bg-gradient-to-r ${colorScheme.gradient} bg-clip-text text-transparent`}>
                                {charmStats.total_transactions || '0'}
                            </div>
                        </div>
                        <div className={`bg-dark-800/50 p-4 rounded-lg ${colorScheme.hover} transition-colors`}>
                            <div className="text-sm text-gray-400 mb-1">Confirmed Transactions</div>
                            <div className={`text-2xl font-bold bg-gradient-to-r ${colorScheme.gradient} bg-clip-text text-transparent`}>
                                {charmStats.confirmed_transactions || '0'}
                            </div>
                        </div>
                        <div className={`bg-dark-800/50 p-4 rounded-lg ${colorScheme.hover} transition-colors`}>
                            <div className="text-sm text-gray-400 mb-1">Confirmation Rate</div>
                            <div className={`text-2xl font-bold bg-gradient-to-r ${colorScheme.gradient} bg-clip-text text-transparent`}>
                                {charmStats.total_transactions > 0
                                    ? `${((charmStats.confirmed_transactions / charmStats.total_transactions) * 100).toFixed(1)}%`
                                    : '0%'}
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            {/* Asset Types Card */}
            <div className="bg-gradient-to-br from-dark-800 to-dark-900 rounded-lg overflow-hidden shadow-lg">
                <div className="p-6">
                    <h2 className="text-xl font-semibold text-white mb-4">
                        Charms by Asset Type
                        <span className="ml-2 text-sm font-normal text-dark-400">
                            {networkType === 'mainnet' ? '(Mainnet)' : '(Testnet 4)'}
                        </span>
                    </h2>
                    <div className="overflow-x-auto max-h-[200px]">
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableCell>Type</TableCell>
                                    <TableCell>Count</TableCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {charmStats.charms_by_asset_type && charmStats.charms_by_asset_type.length > 0 ? (
                                    charmStats.charms_by_asset_type.map((assetType, index) => (
                                        <TableRow key={index}>
                                            <TableCell className="capitalize">{assetType.asset_type}</TableCell>
                                            <TableCell>{formatNumber(assetType.count)}</TableCell>
                                        </TableRow>
                                    ))
                                ) : (
                                    <TableRow>
                                        <TableCell colSpan={2} className="text-center">No data available</TableCell>
                                    </TableRow>
                                )}
                            </TableBody>
                        </Table>
                    </div>
                </div>
            </div>

            {/* Tag Statistics Card - DEX & Token Stats */}
            <div className="bg-gradient-to-br from-dark-800 to-dark-900 rounded-lg overflow-hidden shadow-lg md:col-span-2">
                <div className="p-6">
                    <h2 className="text-xl font-semibold text-white mb-4">
                        DEX & Token Statistics
                        <span className="ml-2 text-sm font-normal text-dark-400">
                            {networkType === 'mainnet' ? '(Mainnet)' : '(Testnet 4)'}
                        </span>
                    </h2>
                    <div className="grid grid-cols-2 gap-4">
                        <div className={`bg-dark-800/50 p-4 rounded-lg ${colorScheme.hover} transition-colors`}>
                            <div className="text-sm text-gray-400 mb-1">Charms Cast Orders</div>
                            <div className={`text-2xl font-bold bg-gradient-to-r from-purple-400 to-purple-600 bg-clip-text text-transparent`}>
                                {formatNumber(tagStats.charms_cast_count)}
                            </div>
                            <div className="text-xs text-gray-500 mt-1">DEX swap orders</div>
                        </div>
                        <div className={`bg-dark-800/50 p-4 rounded-lg ${colorScheme.hover} transition-colors`}>
                            <div className="text-sm text-gray-400 mb-1">$BRO Token Txs</div>
                            <div className={`text-2xl font-bold bg-gradient-to-r from-yellow-400 to-yellow-600 bg-clip-text text-transparent`}>
                                {formatNumber(tagStats.bro_count)}
                            </div>
                            <div className="text-xs text-gray-500 mt-1">BRO token transactions</div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
