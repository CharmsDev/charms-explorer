'use client';

import { ENDPOINTS } from '../apiConfig';

const IDLE_FALLBACK_NETWORK = {
    indexer_status: {
        status: "idle",
        last_processed_block: 0,
        latest_confirmed_block: 0,
        last_updated_at: "Unavailable",
        last_indexer_loop_time: "Unavailable"
    },
    bitcoin_node: {
        status: "unknown",
        network: "",
        block_count: 0,
        best_block_hash: "unknown"
    },
    charm_stats: {
        total_charms: 0,
        total_transactions: 0,
        confirmed_transactions: 0,
        confirmation_rate: 0,
        charms_by_asset_type: []
    },
    tag_stats: {
        charms_cast_count: 0,
        bro_count: 0,
        dex_orders_count: 0
    }
};

export const fetchIndexerStatus = async () => {
    try {
        const response = await fetch(ENDPOINTS.STATUS);

        if (!response.ok) {
            // API returned an error — return idle fallback
            return {
                networks: {
                    testnet4: { ...IDLE_FALLBACK_NETWORK, bitcoin_node: { ...IDLE_FALLBACK_NETWORK.bitcoin_node, network: "testnet4" } },
                    mainnet: { ...IDLE_FALLBACK_NETWORK, bitcoin_node: { ...IDLE_FALLBACK_NETWORK.bitcoin_node, network: "mainnet" } }
                }
            };
        }

        const data = await response.json();
        return data;
    } catch (error) {
        // Network error or timeout — return idle fallback
        return {
            networks: {
                testnet4: { ...IDLE_FALLBACK_NETWORK, bitcoin_node: { ...IDLE_FALLBACK_NETWORK.bitcoin_node, network: "testnet4" } },
                mainnet: { ...IDLE_FALLBACK_NETWORK, bitcoin_node: { ...IDLE_FALLBACK_NETWORK.bitcoin_node, network: "mainnet" } }
            }
        };
    }
};

