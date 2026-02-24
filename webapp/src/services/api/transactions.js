'use client';

import { ENDPOINTS } from '../apiConfig';
import { logger } from '../apiUtils';

export const fetchTransactions = async (page = 1, limit = 50, sort = 'newest', network = null) => {
    try {
        let url = `${ENDPOINTS.TRANSACTIONS}`;
        const params = new URLSearchParams();

        params.append('page', page.toString());
        params.append('limit', limit.toString());
        params.append('sort', sort);

        if (network) {
            params.append('network', network);
        }

        url += `?${params.toString()}`;

        const response = await fetch(url);

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const data = await response.json();

        if (!data.data || !data.data.transactions) {
            logger.warn('fetchTransactions', 'No transactions data in response');
            return {
                transactions: [],
                total: 0,
                page: 1,
                totalPages: 1
            };
        }

        const transactions = data.data.transactions;
        const totalPages = data.pagination?.total_pages || Math.ceil(data.pagination?.total / limit) || 1;
        const totalCount = data.pagination?.total || transactions.length;

        return {
            transactions,
            total: totalCount,
            page: page,
            totalPages: totalPages
        };
    } catch (error) {
        logger.error('fetchTransactions', error);
        throw error;
    }
};
