'use client';

import { useState } from 'react';
import Link from 'next/link';

export default function CharmCard({ charm }) {
    const [imageError, setImageError] = useState(false);
    const placeholderImage = "https://charms.dev/_astro/logo-charms-dark.Ceshk2t3.png";

    // Determine if the charm is an NFT
    const isNftCharm = charm.type === 'nft';

    // Format date
    const formattedDate = new Date(charm.createdAt).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric'
    });

    return (
        <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden shadow-sm hover:shadow-md transition-shadow flex flex-col h-full">
            {/* Image section */}
            <Link href={`/asset/${charm.id}`} className="block w-full h-48 bg-gray-100 dark:bg-gray-900 overflow-hidden">
                <img
                    src={!imageError ? charm.image : placeholderImage}
                    alt={charm.name}
                    className="w-full h-full object-cover"
                    onError={() => setImageError(true)}
                />
            </Link>

            <div className="p-4 flex-grow flex flex-col">
                <div className="flex justify-between items-start">
                    <div>
                        <Link href={`/asset/${charm.id}`} className="hover:text-blue-500">
                            <h3 className="font-medium text-gray-900 dark:text-white">
                                {charm.name}
                            </h3>
                        </Link>
                        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                            {isNftCharm ? 'NFT' :
                                charm.type === 'token' ? 'Token' :
                                    charm.type === 'dapp' ? 'dApp' :
                                        charm.type === 'spell' ? 'Spell' :
                                            charm.type}
                        </p>
                    </div>
                    {charm.type === 'token' && (
                        <div className="text-right">
                            <span className="text-lg font-semibold dark:text-white">{charm.remaining}</span>
                            <p className="text-xs text-gray-500 dark:text-gray-400">{charm.ticker}</p>
                        </div>
                    )}
                </div>

                {/* Description section */}
                {charm.description && (
                    <div className="mt-3">
                        <p className="text-sm text-gray-600 dark:text-gray-300 line-clamp-2">{charm.description}</p>
                    </div>
                )}

                {/* URL section for dApps */}
                {charm.url && (
                    <div className="mt-2">
                        <a
                            href={charm.url}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-sm text-blue-500 hover:underline"
                        >
                            Visit website
                        </a>
                    </div>
                )}

                {/* Stats section */}
                <div className="mt-4 pt-3 border-t border-gray-100 dark:border-gray-700 flex items-center justify-between">
                    <div className="flex items-center space-x-4">
                        <div className="flex items-center">
                            <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4 text-gray-400 dark:text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z" />
                            </svg>
                            <span className="ml-1 text-sm text-gray-500 dark:text-gray-400">{charm.likes}</span>
                        </div>
                        <div className="flex items-center">
                            <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4 text-gray-400 dark:text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                            </svg>
                            <span className="ml-1 text-sm text-gray-500 dark:text-gray-400">{charm.comments}</span>
                        </div>
                    </div>
                    <div className="text-xs text-gray-500 dark:text-gray-400">{formattedDate}</div>
                </div>

                {/* UTXO ID (shortened) */}
                <div className="mt-2 text-xs text-gray-500 dark:text-gray-400 font-mono truncate">
                    {charm.txid.substring(0, 8)}...:{charm.outputIndex}
                </div>
            </div>
        </div>
    );
}
