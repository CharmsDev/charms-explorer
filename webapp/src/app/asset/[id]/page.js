'use client';

export const runtime = 'edge';

import React, { useState, useEffect, useRef } from 'react';
import Link from 'next/link';
import { useParams } from 'next/navigation';
import { getAssetById } from '../../../services/api';
import HoldersTab from '../../../components/HoldersTab'; // [RJJ-STATS-HOLDERS]

// Simple function to attempt hash verification
// This is a simplified approach that just checks if verification is possible
// and returns an appropriate status message
async function attemptHashVerification(imageUrl) {
    try {
        // Try to fetch the image
        const response = await fetch(imageUrl, {
            cache: 'no-store',
            mode: 'no-cors' // This will prevent CORS errors but also make response opaque
        });

        // If we get here, the image exists but we can't verify the hash due to CORS
        return {
            status: 'cors-error',
            message: 'Cannot verify hash: Cross-origin resource sharing (CORS) restriction',
            error: null
        };
    } catch (error) {
        // If we can't even fetch the image
        return {
            status: 'fetch-error',
            message: 'Cannot fetch image for verification',
            error: error.message
        };
    }
}

export default function AssetDetailPage() {
    const { id } = useParams();
    const [asset, setAsset] = useState(null);
    const [isLoading, setIsLoading] = useState(true);
    const [imageError, setImageError] = useState(false);
    const [hashVerification, setHashVerification] = useState({ status: 'pending', computedHash: null });
    const [activeTab, setActiveTab] = useState('details'); // [RJJ-STATS-HOLDERS] Tab state
    const placeholderImage = "/images/logo.png";
    const imageRef = React.useRef(null);

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

    // Attempt hash verification when asset is loaded and not in error state
    useEffect(() => {
        const verifyImageHash = async () => {
            if (asset && asset.imageHash && asset.image && !imageError) {
                try {
                    setHashVerification({ status: 'verifying', computedHash: null, message: null });

                    // Attempt verification
                    const result = await attemptHashVerification(asset.image);

                    // Update state based on result
                    setHashVerification({
                        status: 'cors-error',
                        computedHash: null,
                        message: result.message
                    });
                } catch (error) {
                    console.error('Error verifying image hash:', error);
                    setHashVerification({
                        status: 'error',
                        computedHash: null,
                        message: 'Error verifying hash: ' + error.message
                    });
                }
            } else if (!asset?.imageHash) {
                setHashVerification({ status: 'not-available', computedHash: null, message: null });
            }
        };

        verifyImageHash();
    }, [asset, imageError]);

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
                <div className="mb-8 bg-gray-100 dark:bg-gray-800 rounded-lg overflow-hidden flex justify-center">
                    <img
                        src={!imageError ? asset.image : placeholderImage}
                        alt={asset.name}
                        className="h-64 w-auto object-contain"
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

                {/* [RJJ-STATS-HOLDERS] Tabs Navigation */}
                <div className="mb-8">
                    <div className="border-b border-gray-200 dark:border-gray-700">
                        <nav className="-mb-px flex space-x-8">
                            <button
                                onClick={() => setActiveTab('details')}
                                className={`py-4 px-1 border-b-2 font-medium text-sm transition-colors ${activeTab === 'details'
                                    ? 'border-orange-500 text-orange-600'
                                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                                    }`}
                            >
                                Details
                            </button>
                            <button
                                onClick={() => setActiveTab('holders')}
                                className={`py-4 px-1 border-b-2 font-medium text-sm transition-colors ${activeTab === 'holders'
                                    ? 'border-orange-500 text-orange-600'
                                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                                    }`}
                            >
                                Holders
                            </button>
                        </nav>
                    </div>

                    {/* Tab Content */}
                    <div className="mt-6">
                        {activeTab === 'details' && (
                            <div>
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
                                                <div className="text-2xl font-semibold">{asset.supply ? asset.supply.toLocaleString() : '0'}</div>
                                            </div>
                                            <div className="bg-gray-100 dark:bg-gray-800 rounded-lg p-4">
                                                <div className="text-sm text-gray-500 dark:text-gray-400">Remaining</div>
                                                <div className="text-2xl font-semibold">{asset.remaining ? asset.remaining.toLocaleString() : '0'}</div>
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
                                            {asset.utxoId && (
                                                <div>
                                                    <div className="text-sm text-gray-500 dark:text-gray-400">Input UTXO ID</div>
                                                    <div className="font-mono text-sm break-all">{asset.utxoId}</div>
                                                </div>
                                            )}
                                            <div>
                                                <div className="text-sm text-gray-500 dark:text-gray-400">Address</div>
                                                <div className="font-mono text-sm break-all">{asset.address}</div>
                                            </div>
                                            {asset.version && (
                                                <div>
                                                    <div className="text-sm text-gray-500 dark:text-gray-400">Version</div>
                                                    <div className="font-mono text-sm">{asset.version}</div>
                                                </div>
                                            )}
                                        </div>
                                    </div>
                                </div>

                                {/* Charm Metadata */}
                                {(asset.imageHash || asset.appData) && (
                                    <div className="mb-8">
                                        <h2 className="text-xl font-semibold mb-3">Additional Metadata</h2>
                                        <div className="bg-gray-100 dark:bg-gray-800 rounded-lg p-4">
                                            <div className="grid grid-cols-1 gap-4">
                                                {asset.imageHash && (
                                                    <div>
                                                        <div className="flex items-center justify-between">
                                                            <div className="text-sm text-gray-500 dark:text-gray-400">Image Hash</div>
                                                            {/* Hash verification status indicator */}
                                                            {hashVerification.status === 'pending' && (
                                                                <span className="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded">
                                                                    Pending verification...
                                                                </span>
                                                            )}
                                                            {hashVerification.status === 'verifying' && (
                                                                <span className="px-2 py-1 text-xs bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 rounded">
                                                                    Verifying...
                                                                </span>
                                                            )}
                                                            {hashVerification.status === 'verified' && (
                                                                <span className="px-2 py-1 text-xs bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-300 rounded flex items-center">
                                                                    <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M5 13l4 4L19 7"></path>
                                                                    </svg>
                                                                    Verified
                                                                </span>
                                                            )}
                                                            {hashVerification.status === 'failed' && (
                                                                <span className="px-2 py-1 text-xs bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300 rounded flex items-center">
                                                                    <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M6 18L18 6M6 6l12 12"></path>
                                                                    </svg>
                                                                    Hash mismatch
                                                                </span>
                                                            )}
                                                            {hashVerification.status === 'error' && (
                                                                <span className="px-2 py-1 text-xs bg-yellow-100 dark:bg-yellow-900 text-yellow-700 dark:text-yellow-300 rounded">
                                                                    Verification error
                                                                </span>
                                                            )}
                                                        </div>
                                                        <div className="font-mono text-sm break-all">{asset.imageHash}</div>

                                                        {/* Display verification message if available */}
                                                        {hashVerification.message && (
                                                            <div className="mt-2 text-sm text-gray-500 dark:text-gray-400 italic">
                                                                {hashVerification.message}
                                                            </div>
                                                        )}

                                                        {/* Verify button to manually trigger verification */}
                                                        <button
                                                            onClick={async () => {
                                                                if (asset && asset.imageHash && asset.image && !imageError) {
                                                                    try {
                                                                        setHashVerification({
                                                                            status: 'verifying',
                                                                            computedHash: null,
                                                                            message: null
                                                                        });

                                                                        // Attempt verification
                                                                        const result = await attemptHashVerification(asset.image);

                                                                        // Update state based on result
                                                                        setHashVerification({
                                                                            status: result.status,
                                                                            computedHash: null,
                                                                            message: result.message
                                                                        });
                                                                    } catch (error) {
                                                                        console.error('Error verifying image hash:', error);
                                                                        setHashVerification({
                                                                            status: 'error',
                                                                            computedHash: null,
                                                                            message: 'Error verifying hash: ' + error.message
                                                                        });
                                                                    }
                                                                }
                                                            }}
                                                            className="mt-2 px-3 py-1 text-xs bg-indigo-600 hover:bg-indigo-700 text-white rounded transition-colors"
                                                        >
                                                            Verify Hash
                                                        </button>
                                                    </div>
                                                )}
                                                {asset.appData && (
                                                    <div>
                                                        <div className="text-sm text-gray-500 dark:text-gray-400">App Data</div>
                                                        <div className="font-mono text-sm break-all">{asset.appData}</div>
                                                    </div>
                                                )}
                                            </div>
                                        </div>
                                    </div>
                                )}
                            </div>
                        )}

                        {/* [RJJ-STATS-HOLDERS] Holders Tab */}
                        {activeTab === 'holders' && (
                            <HoldersTab appId={asset.appId} />
                        )}
                    </div>
                </div>

                {/* Social stats */}
                <div className="flex items-center justify-between border-t border-gray-200 dark:border-gray-700 pt-6">
                    <div className="flex items-center space-x-6">

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
