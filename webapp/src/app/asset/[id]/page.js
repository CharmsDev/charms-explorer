'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';
import { useParams } from 'next/navigation';
import { getAssetById } from '../../../services/api';

export default function AssetDetailPage() {
    const { id } = useParams();
    const [asset, setAsset] = useState(null);
    const [isLoading, setIsLoading] = useState(true);
    const [imageError, setImageError] = useState(false);
    const placeholderImage = "https://charms.dev/_astro/logo-charms-dark.Ceshk2t3.png";

    useEffect(() => {
        const loadAsset = async () => {
            try {
                setIsLoading(true);
                const data = await getAssetById(id);
                setAsset(data);
            } catch (error) {
                console.error('Error loading asset:', error);
            } finally {
                setIsLoading(false);
            }
        };

        if (id) {
            loadAsset();
        }
    }, [id]);

    if (isLoading) {
        return (
            <div className="container mx-auto px-4 py-12">
                <div className="max-w-4xl mx-auto">
                    <div className="animate-pulse">
                        <div className="h-8 bg-gray-200 dark:bg-gray-700 rounded w-1/3 mb-6"></div>
                        <div className="h-96 bg-gray-200 dark:bg-gray-700 rounded mb-6"></div>
                        <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-full mb-2"></div>
                        <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-full mb-2"></div>
                        <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-3/4 mb-6"></div>
                        <div className="h-10 bg-gray-200 dark:bg-gray-700 rounded w-1/4"></div>
                    </div>
                </div>
            </div>
        );
    }

    if (!asset) {
        return (
            <div className="container mx-auto px-4 py-12 text-center">
                <h1 className="text-2xl font-bold mb-4">Asset Not Found</h1>
                <p className="mb-6">The asset you're looking for doesn't exist or has been removed.</p>
                <Link href="/" className="bg-indigo-600 text-white px-4 py-2 rounded-md hover:bg-indigo-700 transition-colors">
                    Return to Home
                </Link>
            </div>
        );
    }

    // Format date
    const formattedDate = new Date(asset.createdAt).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'long',
        day: 'numeric'
    });

    // Determine asset type label
    const typeLabel = asset.type === 'nft' ? 'NFT' : asset.type === 'token' ? 'Token' : 'dApp';

    // Determine color theme based on type
    const colorTheme = {
        nft: 'purple',
        token: 'blue',
        dapp: 'green'
    }[asset.type] || 'indigo';

    return (
        <div className="container mx-auto px-4 py-12">
            <div className="max-w-4xl mx-auto">
                {/* Breadcrumb */}
                <div className="flex items-center text-sm text-gray-500 dark:text-gray-400 mb-6">
                    <Link href="/" className="hover:text-indigo-600 dark:hover:text-indigo-400">Home</Link>
                    <span className="mx-2">/</span>
                    <Link
                        href={`/${asset.type === 'nft' ? 'nfts' : asset.type === 'token' ? 'tokens' : 'dapps'}`}
                        className="hover:text-indigo-600 dark:hover:text-indigo-400"
                    >
                        {asset.type === 'nft' ? 'NFTs' : asset.type === 'token' ? 'Tokens' : 'dApps'}
                    </Link>
                    <span className="mx-2">/</span>
                    <span className="font-medium text-gray-700 dark:text-gray-300">{asset.name}</span>
                </div>

                {/* Asset header */}
                <div className="flex flex-col md:flex-row justify-between items-start mb-8">
                    <div>
                        <h1 className="text-3xl font-bold mb-2">{asset.name}</h1>
                        <div className="flex items-center">
                            <span className={`inline-block px-3 py-1 rounded-full text-xs font-medium bg-${colorTheme}-100 text-${colorTheme}-800 dark:bg-${colorTheme}-900 dark:text-${colorTheme}-200 mr-3`}>
                                {typeLabel}
                            </span>
                            {asset.type === 'token' && (
                                <span className="text-gray-600 dark:text-gray-400 font-mono">
                                    {asset.ticker}
                                </span>
                            )}
                            <span className="ml-auto text-gray-500 dark:text-gray-400 text-sm">
                                Created on {formattedDate}
                            </span>
                        </div>
                    </div>
                </div>

                {/* Asset image */}
                <div className="mb-8 bg-gray-100 dark:bg-gray-800 rounded-lg overflow-hidden">
                    <img
                        src={!imageError ? asset.image : placeholderImage}
                        alt={asset.name}
                        className="w-full max-h-96 object-contain"
                        onError={() => setImageError(true)}
                    />
                </div>

                {/* Asset description */}
                {asset.description && (
                    <div className="mb-8">
                        <h2 className="text-xl font-semibold mb-3">Description</h2>
                        <p className="text-gray-700 dark:text-gray-300">{asset.description}</p>
                    </div>
                )}

                {/* Asset attributes for NFTs */}
                {asset.type === 'nft' && asset.attributes && asset.attributes.length > 0 && (
                    <div className="mb-8">
                        <h2 className="text-xl font-semibold mb-3">Attributes</h2>
                        <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
                            {asset.attributes.map((attr, index) => (
                                <div key={index} className="bg-gray-100 dark:bg-gray-800 rounded-lg p-3">
                                    <div className="text-sm text-gray-500 dark:text-gray-400">{attr.trait_type}</div>
                                    <div className="font-medium">{attr.value}</div>
                                </div>
                            ))}
                        </div>
                    </div>
                )}

                {/* Token supply info */}
                {asset.type === 'token' && (
                    <div className="mb-8">
                        <h2 className="text-xl font-semibold mb-3">Token Information</h2>
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div className="bg-gray-100 dark:bg-gray-800 rounded-lg p-4">
                                <div className="text-sm text-gray-500 dark:text-gray-400">Total Supply</div>
                                <div className="text-2xl font-semibold">{asset.supply.toLocaleString()}</div>
                            </div>
                            <div className="bg-gray-100 dark:bg-gray-800 rounded-lg p-4">
                                <div className="text-sm text-gray-500 dark:text-gray-400">Remaining</div>
                                <div className="text-2xl font-semibold">{asset.remaining.toLocaleString()}</div>
                            </div>
                        </div>
                    </div>
                )}

                {/* dApp URL */}
                {asset.type === 'dapp' && asset.url && (
                    <div className="mb-8">
                        <h2 className="text-xl font-semibold mb-3">Application</h2>
                        <a
                            href={asset.url}
                            target="_blank"
                            rel="noopener noreferrer"
                            className={`inline-block bg-${colorTheme}-600 hover:bg-${colorTheme}-700 text-white font-medium py-2 px-4 rounded-md transition-colors`}
                        >
                            Visit dApp
                        </a>
                    </div>
                )}

                {/* Technical details */}
                <div className="mb-8">
                    <h2 className="text-xl font-semibold mb-3">Technical Details</h2>
                    <div className="bg-gray-100 dark:bg-gray-800 rounded-lg p-4">
                        <div className="grid grid-cols-1 gap-4">
                            <div>
                                <div className="text-sm text-gray-500 dark:text-gray-400">ID</div>
                                <div className="font-mono text-sm break-all">{asset.id}</div>
                            </div>
                            <div>
                                <div className="text-sm text-gray-500 dark:text-gray-400">UTXO</div>
                                <div className="font-mono text-sm break-all">{asset.txid}:{asset.outputIndex}</div>
                            </div>
                            <div>
                                <div className="text-sm text-gray-500 dark:text-gray-400">Address</div>
                                <div className="font-mono text-sm break-all">{asset.address}</div>
                            </div>
                        </div>
                    </div>
                </div>

                {/* Social stats */}
                <div className="flex items-center justify-between border-t border-gray-200 dark:border-gray-700 pt-6">
                    <div className="flex items-center space-x-6">
                        <div className="flex items-center">
                            <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5 text-gray-400 dark:text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z" />
                            </svg>
                            <span className="ml-2 text-gray-600 dark:text-gray-400">{asset.likes} likes</span>
                        </div>
                        <div className="flex items-center">
                            <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5 text-gray-400 dark:text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                            </svg>
                            <span className="ml-2 text-gray-600 dark:text-gray-400">{asset.comments} comments</span>
                        </div>
                    </div>
                    <Link
                        href={`/${asset.type === 'nft' ? 'nfts' : asset.type === 'token' ? 'tokens' : 'dapps'}`}
                        className="text-indigo-600 hover:text-indigo-800 dark:text-indigo-400 dark:hover:text-indigo-300"
                    >
                        Back to {asset.type === 'nft' ? 'NFTs' : asset.type === 'token' ? 'Tokens' : 'dApps'}
                    </Link>
                </div>
            </div>
        </div>
    );
}
