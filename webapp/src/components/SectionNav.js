'use client';

import { useState } from 'react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useNetwork } from '@/context/NetworkContext';

const TABS = [
    { base: '/', label: 'Charms', key: 'charms' },
    { base: '/transactions', label: 'Transactions', key: 'transactions' },
    { base: '/cast-dex', label: 'Cast Dex', key: 'cast-dex' },
];

export default function SectionNav({ active, rightSlot }) {
    const router = useRouter();
    const { getNetworkParam } = useNetwork();
    const [query, setQuery] = useState('');

    const networkSuffix = () => {
        const net = getNetworkParam();
        return net !== 'all' ? `?network=${net}` : '';
    };

    const handleSearch = (e) => {
        e.preventDefault();
        const q = query.trim();
        if (!q) return;
        if (/^[a-fA-F0-9]{64}$/.test(q)) { router.push(`/tx?txid=${q}`); return; }
        if (/^(bc1|tb1|1|3|m|n)[a-zA-Z0-9]{25,62}$/.test(q)) { router.push(`/address/${q}`); return; }
        if (/^[a-fA-F0-9]{64}:\d+$/.test(q)) { router.push(`/tx?txid=${q.split(':')[0]}`); return; }
        if (/^[tnb]\//.test(q)) { router.push(`/asset/${encodeURIComponent(q)}`); return; }
        router.push(`/address/${q}`);
    };

    return (
        <div className="bg-dark-900/95 backdrop-blur-sm border-b border-dark-800 sticky top-16 z-40">
            <div className="container mx-auto px-4 py-3">
                <div className="flex items-center justify-between gap-4">
                    <div className="flex items-center gap-2 shrink-0">
                        {TABS.map((tab) => (
                            <Link
                                key={tab.key}
                                href={`${tab.base}${networkSuffix()}`}
                                className={`px-4 py-2 rounded-lg text-sm font-medium transition-all ${
                                    active === tab.key
                                        ? 'bg-primary-600 text-white'
                                        : 'bg-dark-800 text-dark-300 hover:bg-dark-700 hover:text-white'
                                }`}
                            >
                                {tab.label}
                            </Link>
                        ))}
                        {rightSlot && <div className="ml-4 text-dark-400 text-sm">{rightSlot}</div>}
                    </div>
                    <form onSubmit={handleSearch} className="flex-1 max-w-2xl">
                        <div className="relative">
                            <input
                                type="text"
                                value={query}
                                onChange={(e) => setQuery(e.target.value)}
                                placeholder="Search by TXID, address, app ID, or charm ID..."
                                className="w-full bg-dark-800 border border-dark-700 text-white rounded-lg py-2.5 px-4 pl-11 pr-20 font-mono text-xs focus:outline-none focus:border-primary-500 transition-all"
                            />
                            <div className="absolute left-4 top-1/2 -translate-y-1/2 text-dark-400">
                                <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
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
    );
}
