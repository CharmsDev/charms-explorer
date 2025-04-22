'use client';

import { useState } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';

export default function Header() {
    const [isConnecting, setIsConnecting] = useState(false);
    const pathname = usePathname();

    const handleConnect = () => {
        setIsConnecting(true);
        // Simulate wallet connection
        setTimeout(() => {
            setIsConnecting(false);
            alert('RJJ-TODO - Wallet connection');
        }, 1000);
    };

    return (
        <header className="bg-gray-900 text-white">
            <div className="container mx-auto px-4 py-3">
                <div className="flex justify-between items-center">
                    {/* Logo and site name */}
                    <div className="flex items-center space-x-4">
                        <Link href="/">
                            <div className="flex items-center">
                                <img
                                    src="https://charms.dev/_astro/logo-charms-dark.Ceshk2t3.png"
                                    alt="Charms Logo"
                                    className="h-10 w-auto"
                                />
                                <span className="ml-2 text-xl font-bold">Explorer</span>
                            </div>
                        </Link>
                    </div>

                    {/* Navigation */}
                    <nav className="hidden md:flex items-center space-x-6">
                        <Link
                            href="/"
                            className={`hover:text-blue-400 transition-colors ${pathname === '/' ? 'text-blue-400 font-medium' : ''
                                }`}
                        >
                            All
                        </Link>
                        <Link
                            href="/nfts"
                            className={`hover:text-blue-400 transition-colors ${pathname === '/nfts' ? 'text-blue-400 font-medium' : ''
                                }`}
                        >
                            NFTs
                        </Link>
                        <Link
                            href="/tokens"
                            className={`hover:text-blue-400 transition-colors ${pathname === '/tokens' ? 'text-blue-400 font-medium' : ''
                                }`}
                        >
                            Tokens
                        </Link>
                        <Link
                            href="/dapps"
                            className={`hover:text-blue-400 transition-colors ${pathname === '/dapps' ? 'text-blue-400 font-medium' : ''
                                }`}
                        >
                            dApps
                        </Link>
                    </nav>

                    {/* Search bar */}
                    <div className="hidden md:block flex-grow mx-6 max-w-md">
                        <div className="relative">
                            <input
                                type="text"
                                placeholder="Search Charms"
                                className="w-full bg-gray-800 text-white rounded-full py-2 px-4 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            />
                            <button className="absolute right-3 top-2.5">
                                <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                                </svg>
                            </button>
                        </div>
                    </div>

                    {/* Connect wallet button */}
                    <button
                        onClick={handleConnect}
                        disabled={isConnecting}
                        className="bg-indigo-600 hover:bg-indigo-700 text-white font-medium py-2 px-4 rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-75"
                    >
                        {isConnecting ? 'Connecting...' : 'Connect Wallet'}
                    </button>
                </div>
            </div>
        </header>
    );
}
