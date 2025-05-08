'use client';

import { useState, useEffect } from 'react';
import { Card, CardHeader, CardBody, CardFooter } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Table, TableHeader, TableBody, TableRow, TableCell } from '@/components/ui/Table';
import { API_BASE_URL } from '@/services/apiConfig';
import { fetchIndexerStatus, resetIndexer } from '@/services/apiServices';

export default function StatusPage() {
    const [data, setData] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const [lastUpdated, setLastUpdated] = useState(new Date());
    const [resetting, setResetting] = useState(false);
    const [resetMessage, setResetMessage] = useState(null);

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
        // Set up auto-refresh every 30 seconds
        const interval = setInterval(fetchData, 30000);
        return () => clearInterval(interval);
    }, []);

    const handleRefresh = () => {
        fetchData();
    };

    const handleReset = async () => {
        try {
            setResetting(true);
            setResetMessage(null);

            const result = await resetIndexer();

            if (result.success) {
                setResetMessage({
                    type: 'success',
                    text: result.message
                });
            } else {
                setResetMessage({
                    type: 'error',
                    text: result.message || 'Failed to reset indexer'
                });
            }

            // Refresh data after reset
            fetchData();
        } catch (error) {
            setResetMessage({
                type: 'error',
                text: error.message || 'An error occurred while resetting the indexer'
            });
        } finally {
            setResetting(false);
        }
    };

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
                    <Button onClick={handleRefresh} className="mt-4">
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
            <div className="flex justify-between items-center mb-6">
                <h1 className="text-3xl font-bold">Indexer Status</h1>
                <div className="flex gap-2">
                    <Button onClick={handleReset} className="flex items-center" variant="destructive" disabled={resetting}>
                        {resetting ? (
                            <div className="animate-spin rounded-full h-4 w-4 border-t-2 border-b-2 border-white mr-2"></div>
                        ) : (
                            <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                            </svg>
                        )}
                        Reset Indexer
                    </Button>
                    <Button onClick={handleRefresh} className="flex items-center">
                        <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                        Refresh
                    </Button>
                </div>
            </div>

            {resetMessage && (
                <div className={`mb-6 p-4 rounded-md ${resetMessage.type === 'success' ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}`}>
                    <div className="flex">
                        <div className="flex-shrink-0">
                            {resetMessage.type === 'success' ? (
                                <svg className="h-5 w-5 text-green-400" viewBox="0 0 20 20" fill="currentColor">
                                    <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                                </svg>
                            ) : (
                                <svg className="h-5 w-5 text-red-400" viewBox="0 0 20 20" fill="currentColor">
                                    <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clipRule="evenodd" />
                                </svg>
                            )}
                        </div>
                        <div className="ml-3">
                            <p className="text-sm font-medium">{resetMessage.text}</p>
                        </div>
                    </div>
                </div>
            )}

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
