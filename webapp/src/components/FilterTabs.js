'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';

export default function FilterTabs({ counts }) {
    const pathname = usePathname();

    // Define the tabs
    const tabs = [
        { name: 'All', href: '/', count: counts?.total || 0 },
        { name: 'NFTs', href: '/nfts', count: counts?.nft || 0 },
        { name: 'Tokens', href: '/tokens', count: counts?.token || 0 },
        { name: 'dApps', href: '/dapps', count: counts?.dapp || 0 }
    ];

    // Helper function to check if a tab is active
    const isActive = (href) => {
        if (href === '/' && pathname === '/') return true;
        if (href !== '/' && pathname.startsWith(href)) return true;
        return false;
    };

    return (
        <div className="bg-gray-100 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
            <div className="container mx-auto px-4">
                <div className="flex overflow-x-auto py-2 space-x-4">
                    {tabs.map((tab) => (
                        <Link
                            key={tab.name}
                            href={tab.href}
                            className={`
                                px-4 py-2 rounded-full whitespace-nowrap flex items-center
                                ${isActive(tab.href)
                                    ? 'bg-indigo-600 text-white'
                                    : 'bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-600'
                                }
                            `}
                        >
                            <span>{tab.name}</span>
                            <span className={`ml-2 px-2 py-0.5 text-xs rounded-full ${isActive(tab.href)
                                ? 'bg-indigo-700 text-white'
                                : 'bg-gray-100 dark:bg-gray-600 text-gray-600 dark:text-gray-300'
                                }`}>
                                {tab.count.toLocaleString()}
                            </span>
                        </Link>
                    ))}
                </div>
            </div>
        </div>
    );
}
