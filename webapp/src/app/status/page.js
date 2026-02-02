"use client";

import { useState, useEffect } from "react";
import { fetchIndexerStatus } from "@/services/apiServices";
import { useNetwork } from "@/context/NetworkContext";

// Import components
import LoadingState from "@/components/status/LoadingState";
import ErrorState from "@/components/status/ErrorState";
import BlockStatusCards from "@/components/status/BlockStatusCards";
import BlockchainVisualization from "@/components/status/BlockchainVisualization";
import StatusCards from "@/components/status/StatusCards";
import CharmStatistics from "@/components/status/CharmStatistics";
import RecentCharms from "@/components/status/RecentCharms";

export default function StatusPage() {
  const [data, setData] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [lastUpdated, setLastUpdated] = useState(new Date());
  const [isHovered, setIsHovered] = useState(null);

  // Store previous valid block counts to prevent regression on RPC failures
  const [previousBlockCounts, setPreviousBlockCounts] = useState({
    testnet4: 0,
    mainnet: 0,
  });

  // Use the network context
  const { selectedNetworks } = useNetwork();

  // Helper function to preserve valid block counts
  const preserveValidBlockCount = (
    networkType,
    newBlockCount,
    bitcoinNodeData,
  ) => {
    const currentPrevious = previousBlockCounts[networkType];

    // If new block count is 0 or less than previous, keep the previous value
    if (
      newBlockCount <= 0 ||
      (currentPrevious > 0 && newBlockCount < currentPrevious)
    ) {
      return {
        ...bitcoinNodeData,
        block_count: currentPrevious,
      };
    }

    // Update previous block count if new value is valid and greater
    if (newBlockCount > currentPrevious) {
      setPreviousBlockCounts((prev) => ({
        ...prev,
        [networkType]: newBlockCount,
      }));
    }

    return bitcoinNodeData;
  };

  const fetchData = async () => {
    try {
      setLoading(true);
      const statusData = await fetchIndexerStatus();

      // Process and organize data by network
      const processedData = {
        testnet4: {
          indexer_status: {},
          bitcoin_node: {},
          charm_stats: {},
          recent_blocks: [],
        },
        mainnet: {
          indexer_status: {},
          bitcoin_node: {},
          charm_stats: {},
          recent_blocks: [],
        },
      };

      // Process data for each network
      if (statusData.networks) {
        // If the API returns data already organized by networks
        const testnet4Raw = statusData.networks.testnet4 || {};
        const mainnetRaw = statusData.networks.mainnet || {};

        // Preserve valid block counts for testnet4
        if (testnet4Raw.bitcoin_node) {
          testnet4Raw.bitcoin_node = preserveValidBlockCount(
            "testnet4",
            testnet4Raw.bitcoin_node.block_count || 0,
            testnet4Raw.bitcoin_node,
          );
        }

        // Preserve valid block counts for mainnet
        if (mainnetRaw.bitcoin_node) {
          mainnetRaw.bitcoin_node = preserveValidBlockCount(
            "mainnet",
            mainnetRaw.bitcoin_node.block_count || 0,
            mainnetRaw.bitcoin_node,
          );
        }

        processedData.testnet4 = testnet4Raw;
        processedData.mainnet = mainnetRaw;
      } else {
        // If the API still returns data in the old format (assume it's testnet4)
        if (statusData.charm_stats) {
          const bitcoinNodeData = statusData.bitcoin_node || {};

          // Preserve valid block count for testnet4 in old format
          const preservedBitcoinNode = preserveValidBlockCount(
            "testnet4",
            bitcoinNodeData.block_count || 0,
            bitcoinNodeData,
          );

          processedData.testnet4 = {
            indexer_status: {
              status: statusData.status,
              last_processed_block: statusData.last_processed_block,
              latest_confirmed_block: statusData.latest_confirmed_block,
              last_updated_at: statusData.last_updated_at,
              last_indexer_loop_time: statusData.last_indexer_loop_time,
            },
            bitcoin_node: preservedBitcoinNode,
            charm_stats: statusData.charm_stats,
            recent_blocks: statusData.recent_blocks || [],
          };
        } else {
          processedData.testnet4 = {
            indexer_status: statusData,
            bitcoin_node: {},
          };
        }
      }

      setData(processedData);
      setLastUpdated(new Date());
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 30000);
    return () => clearInterval(interval);
  }, []);

  const getStatusBadgeClass = (status) => {
    switch (status) {
      case "active":
        return "bg-green-500 text-white";
      case "idle":
        return "bg-yellow-500 text-black";
      case "inactive":
        return "bg-red-500 text-white";
      default:
        return "bg-gray-500 text-white";
    }
  };

  const getConnectionStatusBadgeClass = (status) => {
    return status === "connected"
      ? "bg-green-500 text-white"
      : "bg-red-500 text-white";
  };

  if (loading && !data) {
    return <LoadingState />;
  }

  if (error && !data) {
    return <ErrorState error={error} fetchData={fetchData} />;
  }

  // Helper function to calculate sync progress and blocks behind
  const calculateSyncInfo = (networkData) => {
    const indexerStatus = networkData?.indexer_status || {};
    const bitcoinNode = networkData?.bitcoin_node || {};

    // Get the latest Bitcoin block from the Bitcoin node
    const latestBitcoinBlock = bitcoinNode.block_count || 0;
    const lastProcessedBlock = indexerStatus.last_processed_block || 0;

    // Calculate sync progress based on the latest Bitcoin block
    const syncProgress =
      latestBitcoinBlock > 0 && lastProcessedBlock > 0
        ? Math.min(
            100,
            Math.round((lastProcessedBlock / latestBitcoinBlock) * 100),
          )
        : 0;

    // Calculate blocks behind based on the latest Bitcoin block
    const blocksBehind =
      latestBitcoinBlock > 0 && lastProcessedBlock > 0
        ? latestBitcoinBlock - lastProcessedBlock
        : 0;

    return { syncProgress, blocksBehind };
  };

  // Get data for testnet4 and mainnet
  const testnet4Data = data?.testnet4 || {};
  const mainnetData = data?.mainnet || {};

  // Calculate sync info for both networks
  const testnet4SyncInfo = calculateSyncInfo(testnet4Data);
  const mainnetSyncInfo = calculateSyncInfo(mainnetData);

  // Determine which networks to display based on selectedNetworks from context
  const showTestnet4 = selectedNetworks.bitcoinTestnet4;
  const showMainnet = selectedNetworks.bitcoinMainnet;

  return (
    <div className="container mx-auto px-4 py-8">
      {/* Network Status Sections */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
        {/* Testnet4 Column */}
        {showTestnet4 && (
          <div className={`${showMainnet ? "lg:col-span-1" : "lg:col-span-2"}`}>
            <div className="mb-6">
              <h2 className="text-2xl font-bold bg-gradient-to-r from-blue-400 to-blue-600 bg-clip-text text-transparent">
                Bitcoin Testnet 4 Status
              </h2>
              <div className="h-1 w-20 bg-gradient-to-r from-blue-400 to-blue-600 rounded-full mt-2"></div>
            </div>

            <BlockStatusCards
              indexerStatus={testnet4Data.indexer_status || {}}
              bitcoinStatus={testnet4Data.bitcoin_node || {}}
              blocksBehind={testnet4SyncInfo.blocksBehind}
              syncProgress={testnet4SyncInfo.syncProgress}
              lastUpdated={lastUpdated}
              isHovered={isHovered}
              setIsHovered={setIsHovered}
              networkType="testnet4"
            />

            <BlockchainVisualization
              indexerStatus={testnet4Data.indexer_status || {}}
              charmStats={testnet4Data.charm_stats || {}}
              recentBlocks={testnet4Data.recent_blocks || []}
              networkType="testnet4"
            />

            <StatusCards
              indexerStatus={testnet4Data.indexer_status || {}}
              bitcoinStatus={testnet4Data.bitcoin_node || {}}
              getStatusBadgeClass={getStatusBadgeClass}
              getConnectionStatusBadgeClass={getConnectionStatusBadgeClass}
              lastUpdated={lastUpdated}
              networkType="testnet4"
            />

            <CharmStatistics
              charmStats={testnet4Data.charm_stats || {}}
              tagStats={testnet4Data.tag_stats || {}}
              networkType="testnet4"
            />

            <RecentCharms
              charmStats={testnet4Data.charm_stats || {}}
              networkType="testnet4"
            />
          </div>
        )}

        {/* Mainnet Column */}
        {showMainnet && (
          <div className={`${showTestnet4 ? "lg:col-span-1" : "lg:col-span-2"}`}>
            <div className="mb-6">
              <h2 className="text-2xl font-bold bg-gradient-to-r from-orange-400 to-orange-600 bg-clip-text text-transparent">
                Bitcoin Mainnet Status
              </h2>
              <div className="h-1 w-20 bg-gradient-to-r from-orange-400 to-orange-600 rounded-full mt-2"></div>
            </div>

            <BlockStatusCards
              indexerStatus={mainnetData.indexer_status || {}}
              bitcoinStatus={mainnetData.bitcoin_node || {}}
              blocksBehind={mainnetSyncInfo.blocksBehind}
              syncProgress={mainnetSyncInfo.syncProgress}
              lastUpdated={lastUpdated}
              isHovered={isHovered}
              setIsHovered={setIsHovered}
              networkType="mainnet"
            />

            <BlockchainVisualization
              indexerStatus={mainnetData.indexer_status || {}}
              charmStats={mainnetData.charm_stats || {}}
              recentBlocks={mainnetData.recent_blocks || []}
              networkType="mainnet"
            />

            <StatusCards
              indexerStatus={mainnetData.indexer_status || {}}
              bitcoinStatus={mainnetData.bitcoin_node || {}}
              getStatusBadgeClass={getStatusBadgeClass}
              getConnectionStatusBadgeClass={getConnectionStatusBadgeClass}
              lastUpdated={lastUpdated}
              networkType="mainnet"
            />

            <CharmStatistics
              charmStats={mainnetData.charm_stats || {}}
              tagStats={mainnetData.tag_stats || {}}
              networkType="mainnet"
            />

            <RecentCharms
              charmStats={mainnetData.charm_stats || {}}
              networkType="mainnet"
            />
          </div>
        )}
      </div>
    </div>
  );
}
