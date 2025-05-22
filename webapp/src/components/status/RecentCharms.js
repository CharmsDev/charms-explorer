'use client';

import { Table, TableHeader, TableBody, TableRow, TableCell } from '@/components/ui/Table';

export default function RecentCharms({ charmStats, networkType = 'testnet4' }) {
    // Generate the appropriate mempool URL based on network type
    const getMempoolUrl = (txid) => {
        return networkType === 'mainnet'
            ? `https://mempool.space/tx/${txid}`
            : `https://mempool.space/testnet4/tx/${txid}`;
    };

    // Define color schemes based on network type
    const colorScheme = networkType === 'mainnet'
        ? {
            link: 'text-orange-500 hover:text-orange-700'
        }
        : {
            link: 'text-blue-500 hover:text-blue-700'
        };

    return (
        <div className="bg-gradient-to-br from-dark-800 to-dark-900 rounded-lg overflow-hidden shadow-lg mb-8">
            <div className="p-6">
                <h2 className="text-xl font-semibold text-white mb-4">
                    Recent Charms
                    <span className="ml-2 text-sm font-normal text-dark-400">
                        {networkType === 'mainnet' ? '(Mainnet)' : '(Testnet 4)'}
                    </span>
                </h2>
                <div className="overflow-x-auto">
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableCell className="w-[40%] whitespace-nowrap">TXID</TableCell>
                                <TableCell className="w-[40%] whitespace-nowrap">Charm ID</TableCell>
                                <TableCell className="w-[10%] whitespace-nowrap">Block Height</TableCell>
                                <TableCell className="w-[10%] whitespace-nowrap">Asset Type</TableCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {charmStats.recent_charms && charmStats.recent_charms.length > 0 ? (
                                charmStats.recent_charms.map((charm, index) => (
                                    <TableRow key={index}>
                                        <TableCell className="w-[40%] truncate">
                                            <a
                                                href={charm.mempool_link || getMempoolUrl(charm.txid)}
                                                target="_blank"
                                                rel="noopener noreferrer"
                                                className={`${colorScheme.link} underline`}
                                                title={charm.txid}
                                            >
                                                {charm.txid}
                                            </a>
                                        </TableCell>
                                        <TableCell
                                            className="w-[40%]"
                                            title={charm.charmid}
                                        >
                                            {charm.charmid.length > 15
                                                ? `${charm.charmid.substring(0, 15)}...`
                                                : charm.charmid}
                                        </TableCell>
                                        <TableCell className="w-[10%] whitespace-nowrap">{charm.block_height}</TableCell>
                                        <TableCell className="w-[10%] whitespace-nowrap">{charm.asset_type}</TableCell>
                                    </TableRow>
                                ))
                            ) : (
                                <TableRow>
                                    <TableCell colSpan={4} className="text-center">No recent charms available</TableCell>
                                </TableRow>
                            )}
                        </TableBody>
                    </Table>
                </div>
            </div>
        </div>
    );
}
