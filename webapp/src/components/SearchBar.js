'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';

/**
 * SearchBar component for searching transactions, addresses, and charms
 * Supports: TXID (64 hex chars), Bitcoin addresses (bc1/tb1), Charm IDs
 */
export default function SearchBar({ className = '' }) {
    const [query, setQuery] = useState('');
    const [isSearching, setIsSearching] = useState(false);
    const [error, setError] = useState(null);
    const router = useRouter();

    const detectSearchType = (input) => {
        const trimmed = input.trim();
        
        // TXID: 64 hex characters
        if (/^[a-fA-F0-9]{64}$/.test(trimmed)) {
            return { type: 'txid', value: trimmed };
        }
        
        // Bitcoin address (mainnet: bc1, testnet: tb1, legacy: 1/3/m/n)
        if (/^(bc1|tb1|1|3|m|n)[a-zA-Z0-9]{25,62}$/.test(trimmed)) {
            return { type: 'address', value: trimmed };
        }
        
        // Charm ID format (contains colon like txid:vout)
        if (/^[a-fA-F0-9]{64}:\d+$/.test(trimmed)) {
            return { type: 'charmid', value: trimmed };
        }
        
        // App ID format (t/..., n/..., b/...)
        if (/^[tnb]\//.test(trimmed)) {
            return { type: 'appid', value: trimmed };
        }
        
        // Default: treat as search query
        return { type: 'query', value: trimmed };
    };

    const handleSearch = async (e) => {
        e.preventDefault();
        if (!query.trim()) return;

        setIsSearching(true);
        setError(null);

        try {
            const { type, value } = detectSearchType(query);

            switch (type) {
                case 'txid':
                    router.push(`/tx?txid=${value}`);
                    break;
                case 'address':
                    router.push(`/address/${value}`);
                    break;
                case 'charmid':
                    router.push(`/tx?txid=${value.split(':')[0]}`);
                    break;
                case 'appid':
                    router.push(`/asset?appid=${encodeURIComponent(value)}`);
                    break;
                default:
                    // For general queries, could implement full-text search later
                    setError('Please enter a valid TXID, address, or Charm ID');
            }
        } catch (err) {
            setError('Search failed. Please try again.');
        } finally {
            setIsSearching(false);
        }
    };

    return (
        <div className={`w-full max-w-2xl mx-auto ${className}`}>
            <form onSubmit={handleSearch} className="relative">
                <div className="relative">
                    <input
                        type="text"
                        value={query}
                        onChange={(e) => {
                            setQuery(e.target.value);
                            setError(null);
                        }}
                        placeholder="Search by TXID, address, or Charm ID..."
                        className="w-full px-4 py-3 pl-12 pr-24 bg-dark-800/80 border border-dark-700 rounded-xl text-white placeholder-dark-400 focus:outline-none focus:border-primary-500 focus:ring-1 focus:ring-primary-500 transition-all"
                    />
                    <div className="absolute left-4 top-1/2 -translate-y-1/2 text-dark-400">
                        <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                        </svg>
                    </div>
                    <button
                        type="submit"
                        disabled={isSearching || !query.trim()}
                        className="absolute right-2 top-1/2 -translate-y-1/2 px-4 py-1.5 bg-primary-600 hover:bg-primary-500 disabled:bg-dark-600 disabled:cursor-not-allowed text-white text-sm font-medium rounded-lg transition-colors"
                    >
                        {isSearching ? (
                            <span className="flex items-center gap-2">
                                <span className="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full"></span>
                            </span>
                        ) : (
                            'Search'
                        )}
                    </button>
                </div>
            </form>
            
            {error && (
                <p className="mt-2 text-sm text-red-400 text-center">{error}</p>
            )}
            
            <div className="mt-3 flex flex-wrap justify-center gap-2 text-xs text-dark-400">
                <span className="px-2 py-1 bg-dark-800/50 rounded">TXID</span>
                <span className="px-2 py-1 bg-dark-800/50 rounded">Address</span>
                <span className="px-2 py-1 bg-dark-800/50 rounded">Charm ID</span>
                <span className="px-2 py-1 bg-dark-800/50 rounded">App ID</span>
            </div>
        </div>
    );
}
