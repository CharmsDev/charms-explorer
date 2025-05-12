'use client';

import { useState, useEffect } from 'react';
import { fetchIndexerStatus } from '@/services/apiServices';

// Import components
import PageHeader from '@/components/status/PageHeader';
import LoadingState from '@/components/status/LoadingState';
import ErrorState from '@/components/status/ErrorState';
import BlockStatusCards from '@/components/status/BlockStatusCards';
import BlockchainVisualization from '@/components/status/BlockchainVisualization';
import StatusCards from '@/components/status/StatusCards';
import CharmStatistics from '@/components/status/CharmStatistics';
import RecentCharms from '@/components/status/RecentCharms';

export default function StatusPage() {
    const [data, setData] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const [lastUpdated, setLastUpdated] = useState(new Date());
    const [isHovered, setIsHovered] = useState(null);

    const fetchData = async () => {
        try {
            setLoading(true);
            const statusData = await fetchIndexerStatus();
            console.log('Status data:', statusData);

            if (statusData.charm_stats) {
                setData({
                    indexer_status: {
                        status: statusData.status,
                        last_processed_block: statusData.last_processed_block,
                        latest_confirmed_block: statusData.latest_confirmed_block,
                        last_updated_at: statusData.last_updated_at,
                        last_indexer_loop_time: statusData.last_indexer_loop_time
                    },
                    bitcoin_node: statusData.bitcoin_node || {},
                    charm_stats: statusData.charm_stats,
                    recent_blocks: statusData.recent_blocks || []
                });
            } else {
                setData({
                    indexer_status: statusData,
                    bitcoin_node: {}
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
        const interval = setInterval(fetchData, 10000);
        return () => clearInterval(interval);
    }, []);

    const getStatusBadgeClass = (status) => {
        switch (status) {
            case 'active': return 'bg-green-500 text-white';
            case 'idle': return 'bg-yellow-500 text-black';
            case 'inactive': return 'bg-red-500 text-white';
            default: return 'bg-gray-500 text-white';
        }
    };

    const getConnectionStatusBadgeClass = (status) => {
        return status === 'connected' ? 'bg-green-500 text-white' : 'bg-red-500 text-white';
    };

    if (loading && !data) {
        return <LoadingState />;
    }

    if (error && !data) {
        return <ErrorState error={error} fetchData={fetchData} />;
    }

    const indexerStatus = data?.indexer_status || {};
    const bitcoinStatus = data?.bitcoin_node || {};
    const charmStats = data?.charm_stats || {};

    // Calculate sync progress
    const syncProgress = indexerStatus.latest_confirmed_block && indexerStatus.last_processed_block
        ? Math.min(100, Math.round((indexerStatus.last_processed_block / indexerStatus.latest_confirmed_block) * 100))
        : 0;

    // Calculate blocks behind
    const blocksBehind = indexerStatus.latest_confirmed_block && indexerStatus.last_processed_block
        ? indexerStatus.latest_confirmed_block - indexerStatus.last_processed_block
        : 0;

    return (
        <div className="container mx-auto px-4 py-8">
            <PageHeader />

            <BlockStatusCards
                indexerStatus={indexerStatus}
                bitcoinStatus={bitcoinStatus}
                blocksBehind={blocksBehind}
                syncProgress={syncProgress}
                lastUpdated={lastUpdated}
                isHovered={isHovered}
                setIsHovered={setIsHovered}
            />

            <BlockchainVisualization
                indexerStatus={indexerStatus}
                charmStats={charmStats}
                recentBlocks={data?.recent_blocks || []}
            />

            <StatusCards
                indexerStatus={indexerStatus}
                bitcoinStatus={bitcoinStatus}
                getStatusBadgeClass={getStatusBadgeClass}
                getConnectionStatusBadgeClass={getConnectionStatusBadgeClass}
                lastUpdated={lastUpdated}
            />

            <CharmStatistics charmStats={charmStats} />

            <RecentCharms charmStats={charmStats} />
        </div>
    );
}
