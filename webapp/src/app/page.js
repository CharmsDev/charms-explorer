'use client';

import { useState, useEffect, useRef, Suspense, useCallback } from 'react';
import { useRouter, useSearchParams } from 'next/navigation';
import AssetGrid from '../components/AssetGrid';
import Pagination from '../components/Pagination';
import SectionNav from '../components/SectionNav';
import { fetchAssetsByType, getAssetCounts } from '../services/api';
import { useNetwork } from '../context/NetworkContext';

// Inner component that uses useSearchParams
function HomeContent() {
    const router = useRouter();
    const searchParams = useSearchParams();
    const { getNetworkParam, isHydrated } = useNetwork();
    
    // Read initial state from URL query params
    const initialType = searchParams.get('type') || 'nft';
    const initialPage = parseInt(searchParams.get('page') || '1', 10) || 1;
    const initialSort = searchParams.get('sort') || 'newest';
    const validTypes = ['nft', 'token', 'dapp'];
    const safeInitialType = validTypes.includes(initialType) ? initialType : 'nft';

    const [assets, setAssets] = useState([]);
    const [counts, setCounts] = useState({ total: 0, nft: 0, token: 0, dapp: 0 });
    const [isLoading, setIsLoading] = useState(true);
    const [currentPage, setCurrentPage] = useState(initialPage);
    const [totalPages, setTotalPages] = useState(1);
    const [sortOrder, setSortOrder] = useState(initialSort);
    const [selectedType, setSelectedType] = useState(safeInitialType);

    const ITEMS_PER_PAGE = 12;

    const updateUrl = useCallback((type, page, sort) => {
        const params = new URLSearchParams();
        params.set('type', type);
        if (page > 1) params.set('page', page.toString());
        if (sort !== 'newest') params.set('sort', sort);
        router.replace(`/?${params.toString()}`, { scroll: false });
    }, [router]);

    const loadData = useCallback(async (type, page, sort) => {
        try {
            setIsLoading(true);
            const networkParam = getNetworkParam();
            const apiNetworkParam = networkParam === 'all' ? null : networkParam;

            const [countsData, response] = await Promise.all([
                getAssetCounts(apiNetworkParam),
                fetchAssetsByType(type, page, ITEMS_PER_PAGE, sort, apiNetworkParam),
            ]);
            setCounts(countsData);
            setAssets(response.assets || []);
            setTotalPages(response.totalPages || 1);
        } catch (error) {
            // Error handled silently - UI shows empty state
        } finally {
            setIsLoading(false);
        }
    }, [getNetworkParam]);

    const handleTypeChange = (type) => {
        setSelectedType(type);
        setCurrentPage(1);
        updateUrl(type, 1, sortOrder);
        loadData(type, 1, sortOrder);
    };

    const handleSortChange = (event) => {
        const newSort = event.target.value;
        setSortOrder(newSort);
        setCurrentPage(1);
        updateUrl(selectedType, 1, newSort);
        loadData(selectedType, 1, newSort);
    };

    const handlePageChange = (newPage) => {
        setCurrentPage(newPage);
        updateUrl(selectedType, newPage, sortOrder);
        loadData(selectedType, newPage, sortOrder);
    };

    // Load initial data on hydration, preserving page from URL
    const initialLoadDone = useRef(false);
    useEffect(() => {
        if (!isHydrated) return;
        if (!initialLoadDone.current) {
            // First load: use page from URL
            initialLoadDone.current = true;
            loadData(selectedType, currentPage, sortOrder);
        } else {
            // Network changed: reset to page 1
            setCurrentPage(1);
            updateUrl(selectedType, 1, sortOrder);
            loadData(selectedType, 1, sortOrder);
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
            <SectionNav active="charms" />

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
