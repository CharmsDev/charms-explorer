'use client';

import { Table, TableHeader, TableBody, TableRow, TableCell } from '@/components/ui/Table';

export default function RecentCharms({ charmStats }) {
    return (
        <div className="bg-gradient-to-br from-dark-800 to-dark-900 rounded-lg overflow-hidden shadow-lg mb-8">
            <div className="p-6">
                <h2 className="text-xl font-semibold text-white mb-4">Recent Charms</h2>
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
                                                href={charm.mempool_link}
                                                target="_blank"
                                                rel="noopener noreferrer"
                                                className="text-blue-500 hover:text-blue-700 underline"
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
