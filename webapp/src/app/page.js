'use client';

import { useState, useEffect } from 'react';
import FilterTabs from '../components/FilterTabs';
import AssetGrid from '../components/AssetGrid';
import SearchBar from '../components/SearchBar';
import { fetchAssets, fetchAssetsByType, getCharmsCountByType } from '../services/api';
import { Button } from '../components/ui/Button';

export default function HomePage() {
    const [assets, setAssets] = useState([]);
    const [counts, setCounts] = useState({ total: 0, nft: 0, token: 0, dapp: 0 });
    const [isLoading, setIsLoading] = useState(true);
    const [currentPage, setCurrentPage] = useState(1);
    const [totalPages, setTotalPages] = useState(1);
    const [sortOrder, setSortOrder] = useState('newest');
    const [selectedNetwork, setSelectedNetwork] = useState('all');
    const [selectedType, setSelectedType] = useState('all');
    const [error, setError] = useState(null);

    const ITEMS_PER_PAGE = 12;

    const loadData = async (type = selectedType, network = selectedNetwork, page = currentPage, sort = sortOrder) => {
        try {
            setIsLoading(true);

            // Fetch charm counts if needed
            if (counts.total === 0) {
                const countsData = await getCharmsCountByType(network === 'all' ? null : network);
                setCounts(countsData);
            }

            // Determine network parameter for API call
            const networkParam = network === 'all' ? null : network;

            // Choose the appropriate API call based on type
            let response;
            if (type === 'all') {
                // Use charms endpoint for "All" tab
                response = await fetchAssets(page, ITEMS_PER_PAGE, sort, networkParam);
            } else {
                // Use assets endpoint for specific types (nft, token, dapp)
                response = await fetchAssetsByType(type, page, ITEMS_PER_PAGE, sort, networkParam);
            }
            setAssets(response.assets || []);

            // Force calculation of total pages based on total count
            const totalItems = response.total || counts.total || response.assets?.length || 0;

            // Update counts with the real total from API
            if (response.total) {
                setCounts(prevCounts => ({
                    ...prevCounts,
                    total: response.total
                }));
            }

            // Make sure we have at least 2 pages if we have more than ITEMS_PER_PAGE items
            const calculatedTotalPages = Math.max(
                Math.ceil(totalItems / ITEMS_PER_PAGE),
                totalItems > ITEMS_PER_PAGE ? 2 : 1
            );

            setTotalPages(calculatedTotalPages);
        } catch (error) {
            console.error('Error loading data:', error);
        } finally {
            setIsLoading(false);
        }
    };

    // Handle type change from FilterTabs
    const handleTypeChange = (type) => {
        setSelectedType(type);
        setCurrentPage(1); // Reset to first page when changing type
        loadData(type, selectedNetwork, 1, sortOrder);
    };

    // Handle sort order change
    const handleSortChange = (event) => {
        const newSort = event.target.value;
        setSortOrder(newSort);
        setCurrentPage(1); // Reset to first page when sorting changes
        loadData(selectedType, selectedNetwork, 1, newSort);
    };

    const handleNetworkChange = (network) => {
        setSelectedNetwork(network);
        setCurrentPage(1); // Reset to first page when network changes
        loadData(selectedType, network, 1, sortOrder);
    };

    // Handle pagination
    const handlePageChange = (newPage) => {
        setCurrentPage(newPage);
        loadData(selectedType, selectedNetwork, newPage, sortOrder);
    };

    // Render page numbers for pagination
    const renderPageNumbers = () => {
        const pageNumbers = [];
        const maxVisiblePages = 7;

        let startPage = Math.max(1, currentPage - Math.floor(maxVisiblePages / 2));
        let endPage = Math.min(totalPages, startPage + maxVisiblePages - 1);

        // Adjust start page if we're near the end
        if (endPage - startPage + 1 < maxVisiblePages) {
            startPage = Math.max(1, endPage - maxVisiblePages + 1);
        }

        // Add first page with ellipsis if needed
        if (startPage > 1) {
            pageNumbers.push(
                <Button
                    key={1}
                    onClick={() => handlePageChange(1)}
                    className={`w-8 h-8 p-0 text-sm font-bold ${currentPage === 1 ? 'bg-primary-700 text-white' : 'bg-dark-700 text-dark-200'}`}
                >
                    1
                </Button>
            );

            if (startPage > 2) {
                pageNumbers.push(
                    <span key="ellipsis1" className="px-1">...</span>
                );
            }
        }

        // Add page numbers
        for (let i = startPage; i <= endPage; i++) {
            pageNumbers.push(
                <Button
                    key={i}
                    onClick={() => handlePageChange(i)}
                    className={`w-8 h-8 p-0 text-sm font-bold ${currentPage === i ? 'bg-primary-700 text-white' : 'bg-dark-700 text-dark-200'}`}
                >
                    {i}
                </Button>
            );
        }

        // Add last page with ellipsis if needed
        if (endPage < totalPages) {
            if (endPage < totalPages - 1) {
                pageNumbers.push(
                    <span key="ellipsis2" className="px-1">...</span>
                );
            }

            pageNumbers.push(
                <Button
                    key={totalPages}
                    onClick={() => handlePageChange(totalPages)}
                    className={`w-8 h-8 p-0 text-sm font-bold ${currentPage === totalPages ? 'bg-primary-700 text-white' : 'bg-dark-700 text-dark-200'}`}
                >
                    {totalPages}
                </Button>
            );
        }

        return pageNumbers;
    };

    useEffect(() => {
        loadData();

        window.handleNetworkChange = handleNetworkChange;

        return () => {
            delete window.handleNetworkChange;
        };
    }, []);

    return (
        <div>
            <div className="bg-dark-900 pt-8 pb-6">
                <div className="container mx-auto px-4 text-center">
                    <h1 className="text-4xl font-bold mt-4 mb-3 gradient-text">Explore Charms</h1>
                    <p className="text-xl max-w-2xl mx-auto mb-4 text-dark-200">
                        Discover NFTs, Tokens, and dApps built with Charms technology
                    </p>
                    <SearchBar className="mb-4" />
                </div>
            </div>

            <FilterTabs counts={counts} onTypeChange={handleTypeChange} onNetworkChange={handleNetworkChange} />

            <div className="container mx-auto px-4 py-6">
                <div className="flex justify-between items-center mb-6">
                    <h2 className="text-2xl font-bold">
                        Found <span className="text-primary-500">{counts.total.toLocaleString()}</span> charms
                    </h2>
                    <div className="flex space-x-2">
                        <select
                            className="bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-md px-3 py-2 text-sm"
                            value={sortOrder}
                            onChange={handleSortChange}
                        >
                            <option value="newest">Newest First</option>
                            <option value="oldest">Oldest First</option>
                        </select>
                    </div>
                </div>
            </div>

            <AssetGrid assets={assets} isLoading={isLoading} />

            {!isLoading && (
                <div className="container mx-auto px-4 py-6">
                    <div className="flex flex-col items-center">
                        <div className="text-sm text-dark-400 mb-2">
                            Page {currentPage} of {Math.max(Math.ceil(counts.total / ITEMS_PER_PAGE), 1)}
                        </div>

                        <div className="flex items-center space-x-2 flex-wrap">
                            <Button
                                onClick={() => handlePageChange(1)}
                                disabled={currentPage === 1}
                                className="px-3 py-1"
                            >
                                First
                            </Button>
                            <Button
                                onClick={() => handlePageChange(currentPage - 1)}
                                disabled={currentPage === 1}
                                className="px-3 py-1"
                            >
                                Previous
                            </Button>

                            <div className="flex items-center space-x-1 mx-2 bg-dark-800/50 px-2 py-1 rounded-lg">
                                {renderPageNumbers()}
                            </div>

                            <Button
                                onClick={() => handlePageChange(currentPage + 1)}
                                disabled={currentPage >= Math.ceil(counts.total / ITEMS_PER_PAGE)}
                                className="px-3 py-1"
                            >
                                Next
                            </Button>
                            <Button
                                onClick={() => handlePageChange(Math.ceil(counts.total / ITEMS_PER_PAGE))}
                                disabled={currentPage >= Math.ceil(counts.total / ITEMS_PER_PAGE)}
                                className="px-3 py-1"
                            >
                                Last
                            </Button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}
