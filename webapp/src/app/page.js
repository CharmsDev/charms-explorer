'use client';

import { useState, useEffect, Suspense, useCallback } from 'react';
import { useRouter, useSearchParams } from 'next/navigation';
import Link from 'next/link';
import AssetGrid from '../components/AssetGrid';
import Pagination from '../components/Pagination';
import { fetchUniqueAssets, getUniqueAssetCounts } from '../services/api';
import { useNetwork } from '../context/NetworkContext';

// Inner component that uses useSearchParams
function HomeContent() {
    const router = useRouter();
    const searchParams = useSearchParams();
    const { getNetworkParam, isHydrated } = useNetwork();
    
    // Read initial type from URL query params, default to 'nft'
    const initialType = searchParams.get('type') || 'nft';
    const validTypes = ['nft', 'token', 'dapp'];
    const safeInitialType = validTypes.includes(initialType) ? initialType : 'nft';
    
    const [assets, setAssets] = useState([]);
    const [counts, setCounts] = useState({ total: 0, nft: 0, token: 0, dapp: 0 });
    const [isLoading, setIsLoading] = useState(true);
    const [currentPage, setCurrentPage] = useState(1);
    const [totalPages, setTotalPages] = useState(1);
    const [searchQuery, setSearchQuery] = useState('');
    const [sortOrder, setSortOrder] = useState('newest');
    const [selectedType, setSelectedType] = useState(safeInitialType);

    const ITEMS_PER_PAGE = 12;

    const loadData = useCallback(async (type = selectedType, page = currentPage, sort = sortOrder) => {
        try {
            setIsLoading(true);

            // Get network param from context (mainnet, testnet4, or all)
            const networkParam = getNetworkParam();
            const apiNetworkParam = networkParam === 'all' ? null : networkParam;

            // Fetch unique asset counts
            const countsData = await getUniqueAssetCounts(apiNetworkParam);
            setCounts(countsData);

            // Fetch unique assets (deduplicated by reference)
            const response = await fetchUniqueAssets(type, page, ITEMS_PER_PAGE, sort, apiNetworkParam);
            setAssets(response.assets || []);
            setTotalPages(response.totalPages || 1);
        } catch (error) {
            // Error handled silently - UI shows empty state
        } finally {
            setIsLoading(false);
        }
    }, [selectedType, currentPage, sortOrder, getNetworkParam]);

    // Handle type change from FilterTabs - update URL
    const handleTypeChange = (type) => {
        setSelectedType(type);
        setCurrentPage(1); // Reset to first page when changing type
        
        // Update URL with new type parameter
        const params = new URLSearchParams(searchParams.toString());
        params.set('type', type);
        router.push(`/?${params.toString()}`, { scroll: false });
        
        loadData(type, 1, sortOrder);
    };

    // Handle sort order change
    const handleSortChange = (event) => {
        const newSort = event.target.value;
        setSortOrder(newSort);
        setCurrentPage(1); // Reset to first page when sorting changes
        loadData(selectedType, 1, newSort);
    };

    // Handle search
    const handleSearch = (e) => {
        e.preventDefault();
        const query = searchQuery.trim();
        if (!query) return;

        // TXID: 64 hex characters
        if (/^[a-fA-F0-9]{64}$/.test(query)) {
            router.push(`/tx?txid=${query}`);
            return;
        }
        // Bitcoin address
        if (/^(bc1|tb1|1|3|m|n)[a-zA-Z0-9]{25,62}$/.test(query)) {
            router.push(`/address/${query}`);
            return;
        }
        // Charm ID (txid:vout)
        if (/^[a-fA-F0-9]{64}:\d+$/.test(query)) {
            router.push(`/tx?txid=${query.split(':')[0]}`);
            return;
        }
        // App ID (t/..., n/..., b/...)
        if (/^[tnb]\//.test(query)) {
            router.push(`/asset?appid=${encodeURIComponent(query)}`);
            return;
        }
        // Default: try as address
        router.push(`/address/${query}`);
    };

    // Handle pagination
    const handlePageChange = (newPage) => {
        setCurrentPage(newPage);
        loadData(selectedType, newPage, sortOrder);
    };

    // Load data when hydrated (localStorage loaded) or when network changes
    useEffect(() => {
        if (isHydrated) {
            loadData();
        }
    }, [isHydrated, getNetworkParam]);

    // Filter tabs configuration (no 'All' - only NFTs, Tokens, dApps)
    const filterTabs = [
        { type: 'nft', label: 'NFTs', icon: '🎨', count: counts.nft },
        { type: 'token', label: 'Tokens', icon: '🪙', count: counts.token },
        { type: 'dapp', label: 'dApps', icon: '⚙️', count: counts.dapp },
    ];

    return (
        <div>
            {/* Toolbar: Transactions, Cast Decks, Search */}
            <div className="bg-dark-900/95 backdrop-blur-sm border-b border-dark-800 sticky top-16 z-40">
                <div className="container mx-auto px-4 py-3">
                    <div className="flex items-center justify-between gap-4">
                        {/* Navigation tabs - left */}
                        <div className="flex items-center gap-2">
                            <Link 
                                href="/"
                                className="px-4 py-2 rounded-lg text-sm font-medium bg-primary-600 text-white transition-all"
                            >
                                Charms
                            </Link>
                            <Link 
                                href="/transactions"
                                className="px-4 py-2 rounded-lg text-sm font-medium bg-dark-800 text-dark-300 hover:bg-dark-700 hover:text-white transition-all"
                            >
                                Transactions
                            </Link>
                            <Link 
                                href="/cast-dex"
                                className="px-4 py-2 rounded-lg text-sm font-medium bg-dark-800 text-dark-300 hover:bg-dark-700 hover:text-white transition-all"
                            >
                                Cast Dex
                            </Link>
                        </div>

                        {/* Search bar - right */}
                        <form onSubmit={handleSearch} className="w-96">
                            <div className="relative">
                                <input
                                    type="text"
                                    value={searchQuery}
                                    onChange={(e) => setSearchQuery(e.target.value)}
                                    placeholder="Search TXID, address, charm..."
                                    className="w-full bg-dark-800 border border-dark-700 text-white rounded-lg py-2.5 px-4 pl-11 pr-20 focus:outline-none focus:border-primary-500 transition-all"
                                />
                                <div className="absolute left-4 top-1/2 -translate-y-1/2 text-dark-400">
                                    <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                                    </svg>
                                </div>
                                <button
                                    type="submit"
                                    className="absolute right-1.5 top-1/2 -translate-y-1/2 px-4 py-1.5 bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium rounded-md transition-colors"
                                >
                                    Search
                                </button>
                            </div>
                        </form>
                    </div>
                </div>
            </div>

            {/* Filter tabs row: tabs left, count center-right, sort far right */}
            <div className="container mx-auto px-4 py-3">
                <div className="flex items-center justify-between">
                    {/* Filter tabs - left */}
                    <div className="flex items-center gap-2">
                        {filterTabs.map((tab) => (
                            <button
                                key={tab.type}
                                onClick={() => handleTypeChange(tab.type)}
                                className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-all flex items-center gap-1.5 ${
                                    selectedType === tab.type
                                        ? 'bg-primary-600 text-white'
                                        : 'bg-dark-800 text-dark-300 hover:bg-dark-700 hover:text-white'
                                }`}
                            >
                                <span>{tab.icon}</span>
                                <span className="hidden sm:inline">{tab.label}</span>
                                <span className={`px-1.5 py-0.5 text-xs rounded ${
                                    selectedType === tab.type
                                        ? 'bg-primary-500/30 text-primary-200'
                                        : 'bg-dark-700 text-dark-400'
                                }`}>
                                    {tab.count.toLocaleString()}
                                </span>
                            </button>
                        ))}
                    </div>

                    {/* Count and sort - right */}
                    <div className="flex items-center gap-4">
                        <p className="text-dark-400 text-sm">
                            Found <span className="text-primary-400 font-semibold">{counts.total.toLocaleString()}</span> charms
                        </p>
                        <select
                            className="bg-dark-800 border border-dark-700 rounded-lg px-3 py-1.5 text-sm text-white focus:outline-none focus:border-primary-500"
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
                <Pagination
                    currentPage={currentPage}
                    totalPages={totalPages}
                    totalItems={counts.total}
                    itemsPerPage={ITEMS_PER_PAGE}
                    onPageChange={handlePageChange}
                />
            )}
        </div>
    );
}

// Main export wrapped in Suspense for useSearchParams
export default function HomePage() {
    return (
        <Suspense fallback={
            <div className="container mx-auto px-4 py-8">
                <div className="flex justify-center items-center min-h-[400px]">
                    <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-500"></div>
                </div>
            </div>
        }>
            <HomeContent />
        </Suspense>
    );
}
