'use client';

import { useState, useEffect } from 'react';
import { Card, CardHeader, CardBody, CardFooter } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Table, TableHeader, TableBody, TableRow, TableCell } from '@/components/ui/Table';
import { API_BASE_URL } from '@/services/apiConfig';
import { fetchIndexerStatus } from '@/services/apiServices';

export default function StatusPage() {
    const [data, setData] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const [lastUpdated, setLastUpdated] = useState(new Date());

    const fetchData = async () => {
        try {
            setLoading(true);
            // Try the diagnostic endpoint first
            try {
                const response = await fetch(`${API_BASE_URL}/api/diagnostic`);
                if (response.ok) {
                    const result = await response.json();
                    setData(result);
                    setLastUpdated(new Date());
                    setLoading(false);
                    return;
                }
            } catch (e) {
                console.log('Error fetching diagnostic data, falling back to status endpoint');
            }

            // Fallback to status endpoint
            const statusData = await fetchIndexerStatus();
            console.log('Status data:', statusData);

            // Check if the data is already in the expected format or needs to be restructured
            if (statusData.charm_stats) {
                // Data is already in the expected format with charm_stats
                setData({
                    indexer_status: {
                        status: statusData.status,
                        last_processed_block: statusData.last_processed_block,
                        latest_confirmed_block: statusData.latest_confirmed_block,
                        last_updated_at: statusData.last_updated_at,
                        time_since_last_update: statusData.time_since_last_update
                    },
                    charm_stats: statusData.charm_stats
                });
            } else {
                // Fallback to old format
                setData({
                    indexer_status: statusData
                });
            }
            setLastUpdated(new Date());
        } catch (err) {
            setError(err.message);
            console.error('Error fetching data:', err);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchData();
        // Set up auto-refresh every 10 seconds
        const interval = setInterval(fetchData, 10000);
        return () => clearInterval(interval);
    }, []);

    const getStatusBadgeClass = (status) => {
        switch (status) {
            case 'active':
                return 'bg-green-500 text-white';
            case 'idle':
                return 'bg-yellow-500 text-black';
            case 'inactive':
                return 'bg-red-500 text-white';
            default:
                return 'bg-gray-500 text-white';
        }
    };

    const getConnectionStatusBadgeClass = (status) => {
        return status === 'connected' ? 'bg-green-500 text-white' : 'bg-red-500 text-white';
    };

    if (loading && !data) {
        return (
            <div className="container mx-auto px-4 py-8">
                <h1 className="text-3xl font-bold mb-6">Indexer Status</h1>
                <div className="flex justify-center items-center h-64">
                    <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-500"></div>
                </div>
            </div>
        );
    }

    if (error && !data) {
        return (
            <div className="container mx-auto px-4 py-8">
                <h1 className="text-3xl font-bold mb-6">Indexer Status</h1>
                <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded relative" role="alert">
                    <strong className="font-bold">Error: </strong>
                    <span className="block sm:inline">{error}</span>
                    <p className="mt-2">
                        Could not connect to the indexer. Please make sure the indexer is running and accessible at {API_BASE_URL}.
                    </p>
                    <Button onClick={fetchData} className="mt-4">
                        Try Again
                    </Button>
                </div>
            </div>
        );
    }

    const indexerStatus = data?.indexer_status || {};
    const bitcoinStatus = data?.bitcoin_rpc_test || {};
    const charmStats = data?.charm_stats || {};

    return (
        <div className="container mx-auto px-4 py-8">
            <div className="mb-6">
                <h1 className="text-3xl font-bold">Indexer Status</h1>
            </div>

            {/* Blockchain Visualization */}
            <div className="mb-8 bg-transparent py-6">
                <div className="blockchain-wrapper">
                    <div className="relative">
                        <div className="blockchain-blocks flex justify-center relative">
                            {/* Current Processing Block */}
                            <div className="bitcoin-block text-center mempool-block relative"
                                style={{
                                    width: '150px',
                                    height: '150px',
                                    margin: '0 15px',
                                    borderRadius: '4px',
                                    position: 'relative',
                                    transform: 'perspective(800px) rotateY(-10deg) rotateX(5deg)',
                                    transformStyle: 'preserve-3d',
                                    boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.5), 0 4px 6px -2px rgba(0, 0, 0, 0.3)',
                                    background: 'linear-gradient(135deg, #554b45 0%, #554b45 60%, #3b82f6 60%, #3b82f6 100%)',
                                    transition: 'transform 0.5s ease-in-out'
                                }}>
                                <div className="block-body p-4 text-white" style={{ transform: 'translateZ(10px)' }}>
                                    <div className="text-2xl font-bold text-blue-500 mb-2">
                                        {indexerStatus.last_processed_block || '?'}
                                    </div>
                                    <div className="text-sm mb-1">
                                        {charmStats.total_charms ? `${charmStats.total_charms} charms` : 'No charms'}
                                    </div>
                                    <div className="text-sm mb-1">
                                        {charmStats.total_transactions ? `${charmStats.total_transactions} txs` : 'No transactions'}
                                    </div>
                                    <div className="text-sm">
                                        Processing...
                                    </div>
                                </div>
                                <span className="animated-border absolute inset-0 border-2 border-yellow-400 rounded-md"
                                    style={{
                                        animation: 'pulse 2s infinite',
                                        boxShadow: '0 0 10px rgba(250, 204, 21, 0.5)'
                                    }}></span>
                            </div>

                            {/* Previous Blocks - Generate 5 blocks */}
                            {Array.from({ length: 5 }).map((_, index) => {
                                const blockHeight = indexerStatus.last_processed_block ? (indexerStatus.last_processed_block - (index + 1)) : '?';
                                return (
                                    <div key={index} className="bitcoin-block mined-block text-center relative"
                                        style={{
                                            width: '150px',
                                            height: '150px',
                                            margin: '0 15px',
                                            background: 'linear-gradient(135deg, #1e293b 0%, #2d3748 100%)',
                                            borderRadius: '4px',
                                            transform: 'perspective(800px) rotateY(-10deg) rotateX(5deg)',
                                            transformStyle: 'preserve-3d',
                                            boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.3), 0 4px 6px -2px rgba(0, 0, 0, 0.2)',
                                            transition: 'transform 0.5s ease-in-out'
                                        }}>
                                        <div className="block-body p-4 text-white" style={{ transform: 'translateZ(5px)' }}>
                                            <div className="text-2xl font-bold text-blue-500 mb-2">
                                                {blockHeight}
                                            </div>
                                            <div className="text-sm mb-1">
                                                Confirmed
                                            </div>
                                            <div className="text-sm">
                                                {index === 0 ? 'Just now' : `${index * 10} min ago`}
                                            </div>
                                        </div>
                                    </div>
                                );
                            })}
                        </div>
                    </div>
                </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
                {/* Indexer Status Card */}
                <Card>
                    <CardHeader className="flex justify-between items-center">
                        <h2 className="text-xl font-semibold">Indexer Status</h2>
                        <Badge className={getStatusBadgeClass(indexerStatus.status)}>
                            {indexerStatus.status || 'Unknown'}
                        </Badge>
                    </CardHeader>
                    <CardBody>
                        <div className="space-y-2">
                            <div className="flex justify-between">
                                <span className="font-medium">Last Processed Block:</span>
                                <span>{indexerStatus.last_processed_block || '-'}</span>
                            </div>
                            <div className="flex justify-between">
                                <span className="font-medium">Latest Confirmed Block:</span>
                                <span>{indexerStatus.latest_confirmed_block || '-'}</span>
                            </div>
                            <div className="flex justify-between">
                                <span className="font-medium">Last Updated:</span>
                                <span>{indexerStatus.last_updated_at || '-'}</span>
                            </div>
                            <div className="flex justify-between">
                                <span className="font-medium">Time Since Last Update:</span>
                                <span>{indexerStatus.time_since_last_update || '-'}</span>
                            </div>
                        </div>
                    </CardBody>
                </Card>

                {/* Bitcoin Node Card */}
                <Card>
                    <CardHeader className="flex justify-between items-center">
                        <h2 className="text-xl font-semibold">Bitcoin Node</h2>
                        <Badge className={getConnectionStatusBadgeClass(bitcoinStatus.status)}>
                            {bitcoinStatus.status || 'Unknown'}
                        </Badge>
                    </CardHeader>
                    <CardBody>
                        <div className="space-y-2">
                            <div className="flex justify-between">
                                <span className="font-medium">Host:</span>
                                <span>{bitcoinStatus.host || '-'}</span>
                            </div>
                            <div className="flex justify-between">
                                <span className="font-medium">Port:</span>
                                <span>{bitcoinStatus.port || '-'}</span>
                            </div>
                            <div className="flex justify-between">
                                <span className="font-medium">Current Block Height:</span>
                                <span>{bitcoinStatus.block_count || '-'}</span>
                            </div>
                            <div className="flex justify-between">
                                <span className="font-medium">Best Block Hash:</span>
                                <span className="text-xs truncate max-w-[200px]">{bitcoinStatus.best_block_hash || '-'}</span>
                            </div>
                        </div>
                    </CardBody>
                </Card>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
                {/* Charm Statistics Card */}
                <Card>
                    <CardHeader>
                        <h2 className="text-xl font-semibold">Charm Statistics</h2>
                    </CardHeader>
                    <CardBody>
                        <div className="grid grid-cols-2 gap-4">
                            <div className="bg-dark-800/30 p-4 rounded-lg">
                                <div className="text-sm text-gray-400">Total Charms</div>
                                <div className="text-2xl font-bold">{charmStats.total_charms || '0'}</div>
                            </div>
                            <div className="bg-dark-800/30 p-4 rounded-lg">
                                <div className="text-sm text-gray-400">Total Transactions</div>
                                <div className="text-2xl font-bold">{charmStats.total_transactions || '0'}</div>
                            </div>
                            <div className="bg-dark-800/30 p-4 rounded-lg">
                                <div className="text-sm text-gray-400">Confirmed Transactions</div>
                                <div className="text-2xl font-bold">{charmStats.confirmed_transactions || '0'}</div>
                            </div>
                            <div className="bg-dark-800/30 p-4 rounded-lg">
                                <div className="text-sm text-gray-400">Confirmation Rate</div>
                                <div className="text-2xl font-bold">
                                    {charmStats.total_transactions > 0
                                        ? `${((charmStats.confirmed_transactions / charmStats.total_transactions) * 100).toFixed(1)}%`
                                        : '0%'}
                                </div>
                            </div>
                        </div>
                    </CardBody>
                </Card>

                {/* Asset Types Card */}
                <Card>
                    <CardHeader>
                        <h2 className="text-xl font-semibold">Charms by Asset Type</h2>
                    </CardHeader>
                    <CardBody>
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
                    </CardBody>
                </Card>
            </div>

            {/* Recent Charms Card */}
            <Card>
                <CardHeader>
                    <h2 className="text-xl font-semibold">Recent Charms</h2>
                </CardHeader>
                <CardBody>
                    <div className="overflow-x-auto">
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableCell>TXID</TableCell>
                                    <TableCell>Charm ID</TableCell>
                                    <TableCell>Block Height</TableCell>
                                    <TableCell>Asset Type</TableCell>
                                    <TableCell>Date Created</TableCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {charmStats.recent_charms && charmStats.recent_charms.length > 0 ? (
                                    charmStats.recent_charms.map((charm, index) => (
                                        <TableRow key={index}>
                                            <TableCell className="max-w-[150px] truncate">{charm.txid}</TableCell>
                                            <TableCell className="max-w-[150px] truncate">{charm.charmid}</TableCell>
                                            <TableCell>{charm.block_height}</TableCell>
                                            <TableCell>{charm.asset_type}</TableCell>
                                            <TableCell>{new Date(charm.date_created).toLocaleString()}</TableCell>
                                        </TableRow>
                                    ))
                                ) : (
                                    <TableRow>
                                        <TableCell colSpan={5} className="text-center">No recent charms</TableCell>
                                    </TableRow>
                                )}
                            </TableBody>
                        </Table>
                    </div>
                </CardBody>
                <CardFooter className="text-sm text-gray-400">
                    Last updated: {lastUpdated.toLocaleString()}
                </CardFooter>
            </Card>
        </div>
    );
}
