'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';

export default function Header() {
    const [isConnecting, setIsConnecting] = useState(false);
    const [isScrolled, setIsScrolled] = useState(false);
    const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
    const pathname = usePathname();

    // Handle scroll effect for header
    useEffect(() => {
        const handleScroll = () => {
            if (window.scrollY > 10) {
                setIsScrolled(true);
            } else {
                setIsScrolled(false);
            }
        };

        window.addEventListener('scroll', handleScroll);
        return () => window.removeEventListener('scroll', handleScroll);
    }, []);

    const handleConnect = () => {
        setIsConnecting(true);
        // Simulate wallet connection
        setTimeout(() => {
            setIsConnecting(false);
            alert('RJJ-TODO - Wallet connection');
        }, 1000);
    };

    return (
        <header className={`fixed top-0 left-0 right-0 z-50 border-b transition-all duration-300 ${isScrolled
            ? 'bg-dark-900/80 backdrop-blur-md shadow-md border-transparent'
            : 'bg-transparent border-dark-800/50'
            }`}>
            <div className="container mx-auto px-4 py-4">
                {/* Main header layout with 3 sections */}
                <div className="grid grid-cols-3 items-center">
                    {/* Left section - Logo and site name */}
                    <div className="flex items-center space-x-3">
                        <Link href="/">
                            <div className="flex items-center group">
                                <div className={`relative transition-all duration-300 ${isScrolled ? 'scale-90' : 'scale-100'}`}>
                                    <img
                                        src="https://charms.dev/_astro/logo-charms-dark.Ceshk2t3.png"
                                        alt="Charms Logo"
                                        className="h-7 w-auto group-hover:animate-pulse-slow"
                                    />
                                    <div className="absolute inset-0 rounded-full bg-primary-500/20 blur-md -z-10 opacity-0 group-hover:opacity-100 transition-opacity"></div>
                                </div>
                                <div className="ml-2">
                                    <span className="text-xl font-bold gradient-text">Explorer</span>
                                    <div className="h-0.5 w-0 bg-gradient-to-r from-primary-400 to-primary-600 group-hover:w-full transition-all duration-300"></div>
                                </div>
                            </div>
                        </Link>
                    </div>

                    {/* Center section - Search bar */}
                    <div className="hidden md:flex justify-center">
                        <div className="relative group w-full max-w-md">
                            <input
                                type="text"
                                placeholder="Search Charms"
                                className="w-full bg-dark-800/70 text-white rounded-full py-2 px-4 pl-10 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:bg-dark-800 transition-all"
                            />
                            <div className="absolute left-3 top-2.5 text-dark-400 group-focus-within:text-primary-400 transition-colors">
                                <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                                </svg>
                            </div>
                        </div>
                    </div>

                    {/* Right section - Status button, Connect button and menu */}
                    <div className="flex items-center justify-end space-x-3">
                        {/* Status page button */}
                        <Link
                            href="/status"
                            className="btn btn-secondary"
                        >
                            <span className="flex items-center">
                                <svg className="mr-2 h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"></path>
                                </svg>
                                Status
                            </span>
                        </Link>

                        {/* Connect wallet button */}
                        <button
                            onClick={handleConnect}
                            disabled={isConnecting}
                            className="btn btn-primary shadow-glow"
                        >
                            <span className="flex items-center">
                                {isConnecting ? (
                                    <>
                                        <svg className="animate-spin -ml-1 mr-2 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                                            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                        </svg>
                                        Connecting...
                                    </>
                                ) : (
                                    <>
                                        <svg className="mr-2 h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
                                        </svg>
                                        Connect
                                    </>
                                )}
                            </span>
                        </button>

                        {/* Mobile menu button */}
                        <button
                            className="md:hidden p-2 rounded-lg bg-dark-800/70 hover:bg-dark-700/70 transition-colors"
                            onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                {isMobileMenuOpen ? (
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                                ) : (
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
                                )}
                            </svg>
                        </button>
                    </div>
                </div>
            </div>

            {/* Mobile menu */}
            {isMobileMenuOpen && (
                <div className="md:hidden bg-dark-900/95 backdrop-blur-md border-t border-dark-800">
                    <div className="container mx-auto px-4 py-3">
                        <nav className="flex flex-col space-y-2">
                            {/* Mobile navigation links */}
                            <Link
                                href="/status"
                                className="flex items-center p-2 rounded-lg hover:bg-dark-800 transition-colors"
                            >
                                <svg className="mr-2 h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"></path>
                                </svg>
                                Indexer Status
                            </Link>
                        </nav>
                        <div className="mt-4">
                            <div className="relative">
                                <input
                                    type="text"
                                    placeholder="Search Charms"
                                    className="w-full bg-dark-800/70 text-white rounded-full py-2 px-4 pl-10 focus:outline-none focus:ring-2 focus:ring-primary-500"
                                />
                                <div className="absolute left-3 top-2.5 text-dark-400">
                                    <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                                    </svg>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            )}

            {/* Removed spacer - it will be added in the layout */}
        </header>
    );
}
