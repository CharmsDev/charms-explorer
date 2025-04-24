'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useState, useEffect } from 'react';

export default function FilterTabs({ counts }) {
    const pathname = usePathname();
    const [mounted, setMounted] = useState(false);

    // Set mounted to true on client side
    useEffect(() => setMounted(true), []);

    // Define the tabs with icons
    const tabs = [
        {
            name: 'All',
            href: '/',
            count: counts?.total || 0,
            icon: (
                <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"></path>
                </svg>
            )
        },
        {
            name: 'NFTs',
            href: '/nfts',
            count: counts?.nft || 0,
            icon: (
                <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"></path>
                </svg>
            )
        },
        {
            name: 'Tokens',
            href: '/tokens',
            count: counts?.token || 0,
            icon: (
                <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                </svg>
            )
        },
        {
            name: 'dApps',
            href: '/dapps',
            count: counts?.dapp || 0,
            icon: (
                <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"></path>
                </svg>
            )
        }
    ];

    // Helper function to check if a tab is active
    const isActive = (href) => {
        if (href === '/' && pathname === '/') return true;
        if (href !== '/' && pathname.startsWith(href)) return true;
        return false;
    };

    // Don't render anything until mounted to avoid hydration mismatch
    if (!mounted) return null;

    return (
        <div className="bg-dark-900/90 backdrop-blur-3xl border-y border-dark-800 sticky top-16 z-[100]">
            <div className="container mx-auto px-4">
                <div className="flex overflow-x-auto py-3 space-x-2 scrollbar-hide">
                    {tabs.map((tab) => {
                        const active = isActive(tab.href);
                        return (
                            <Link
                                key={tab.name}
                                href={tab.href}
                                className={`
                                    px-4 py-2 rounded-lg whitespace-nowrap flex items-center transition-all duration-300
                                    ${active
                                        ? 'bg-primary-600/20 text-primary-400 shadow-glow'
                                        : 'bg-dark-800/70 text-dark-300 hover:bg-dark-700/70 hover:text-white'
                                    }
                                `}
                            >
                                {tab.icon}
                                <span>{tab.name}</span>
                                <span className={`ml-2 px-2 py-0.5 text-xs rounded-lg ${active
                                    ? 'bg-primary-600/30 text-primary-300'
                                    : 'bg-dark-700/70 text-dark-400'
                                    }`}>
                                    {tab.count.toLocaleString()}
                                </span>
                            </Link>
                        );
                    })}
                </div>
            </div>
        </div>
    );
}
