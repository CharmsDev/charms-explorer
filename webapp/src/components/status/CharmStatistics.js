'use client';

import { Table, TableHeader, TableBody, TableRow, TableCell } from '@/components/ui/Table';

export default function CharmStatistics({ charmStats, networkType = 'testnet4' }) {
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
                                    <TableCell>Asset Type</TableCell>
                                    <TableCell>Count</TableCell>
                                    <TableCell>Percentage</TableCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {charmStats.charms_by_asset_type && charmStats.charms_by_asset_type.length > 0 ? (
                                    charmStats.charms_by_asset_type.map((assetType, index) => (
                                        <TableRow key={index}>
                                            <TableCell>{assetType.asset_type}</TableCell>
                                            <TableCell>{assetType.count}</TableCell>
                                            <TableCell>
                                                {charmStats.total_charms > 0
                                                    ? `${((assetType.count / charmStats.total_charms) * 100).toFixed(1)}%`
                                                    : '0%'}
                                            </TableCell>
                                        </TableRow>
                                    ))
                                ) : (
                                    <TableRow>
                                        <TableCell colSpan={3} className="text-center">No data available</TableCell>
                                    </TableRow>
                                )}
                            </TableBody>
                        </Table>
                    </div>
                </div>
            </div>
        </div>
    );
}
