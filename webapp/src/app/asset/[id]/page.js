'use client';

export const runtime = 'edge';

import React, { useState, useEffect, useRef } from 'react';
import Link from 'next/link';
import { useParams } from 'next/navigation';
import { getAssetById, fetchAssetByAppId, fetchAssetHolders } from '../../../services/api';
import HoldersTab from '../../../components/HoldersTab'; // [RJJ-STATS-HOLDERS]
import { parseSpellMetadata, getImageSource, formatFieldName, formatFieldValue } from '../../../services/spellParser';
import { fetchReferenceNftByHash, extractHashFromAppId } from '../../../services/api/referenceNft';
import { classifyCharm, getCharmBadgeProps, CHARM_TYPES } from '../../../services/charmClassifier';

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
    const [assetData, setAssetData] = useState(null); // Asset data with supply info
    const [holdersData, setHoldersData] = useState(null); // Holders data with calculated supply
    const [isLoading, setIsLoading] = useState(true);
    const [imageError, setImageError] = useState(false);
    const [hashVerification, setHashVerification] = useState({ status: 'pending', computedHash: null });
    const [activeTab, setActiveTab] = useState('details'); // [RJJ-STATS-HOLDERS] Tab state
    const [nftMetadata, setNftMetadata] = useState(null); // Reference NFT metadata (like AssetCard)
    const [spellImage, setSpellImage] = useState(null); // Image from spell or reference NFT
    const placeholderImage = "/images/logo.png";
    const imageRef = React.useRef(null);

    // Decode the URL-encoded ID
    const decodedId = id ? decodeURIComponent(id) : null;

    useEffect(() => {
        const loadAsset = async () => {
            try {
                setIsLoading(true);
                const data = await getAssetById(decodedId);
                setAsset(data);
                
                const appId = data?.app_id || data?.id || decodedId;
                
                // Fetch asset data (with total_supply) from /assets endpoint
                try {
                    const assetResponse = await fetchAssetByAppId(appId);
                    if (assetResponse) {
                        setAssetData(assetResponse);
                    }
                } catch (e) {
                    console.warn('Could not fetch asset data:', e);
                }
                
                // Fetch holders data (has calculated supply from charms)
                try {
                    const holders = await fetchAssetHolders(appId);
                    if (holders) {
                        setHoldersData(holders);
                    }
                } catch (e) {
                    console.warn('Could not fetch holders data:', e);
                }
                
                // For tokens (t/...), fetch reference NFT metadata for image (like AssetCard)
                if (appId?.startsWith('t/')) {
                    const hash = extractHashFromAppId(appId);
                    if (hash) {
                        const refNft = await fetchReferenceNftByHash(hash);
                        if (refNft) {
                            setNftMetadata(refNft);
                            if (refNft.image_url) {
                                setSpellImage(refNft.image_url);
                            }
                        }
                    }
                }
                // For NFTs (n/...) without image, fetch from reference NFT endpoint
                else if (appId?.startsWith('n/') && !data?.image && !data?.image_url) {
                    const hash = extractHashFromAppId(appId);
                    if (hash) {
                        const refNft = await fetchReferenceNftByHash(hash);
                        if (refNft?.image_url) {
                            setSpellImage(refNft.image_url);
                        }
                    }
                }
            } catch (error) {
                console.error('Error loading asset:', error);
            } finally {
                setIsLoading(false);
            }
        };

        if (decodedId) {
            loadAsset();
        }
    }, [decodedId]);

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
    const formattedDate = new Date(asset.createdAt || assetData?.created_at).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'long',
        day: 'numeric'
    });

    // Determine asset type label
    const typeLabel = asset.type === 'nft' ? 'NFT' : asset.type === 'token' ? 'Token' : 'dApp';

    // Parse spell metadata to get all fields including extra/custom fields
    const spellMetadata = parseSpellMetadata(asset);
    const hasExtraFields = spellMetadata.extraFields && Object.keys(spellMetadata.extraFields).length > 0;
    
    // Get the correct image source (like AssetCard)
    // Priority: nftMetadata.image_url > spellImage > asset.image > asset.image_url > placeholder
    const getDisplayImage = () => {
        if (imageError) return placeholderImage;
        if (nftMetadata?.image_url) return nftMetadata.image_url;
        if (spellImage) return spellImage;
        if (asset.image && asset.image !== '/images/logo.png') return asset.image;
        if (asset.image_url) return asset.image_url;
        return placeholderImage;
    };
    const displayImage = getDisplayImage();

    // Classify charm for badge (like AssetCard)
    const charmType = classifyCharm(asset);
    const isBroToken = charmType === CHARM_TYPES.BRO_TOKEN;

    // Get supply info - prefer holdersData (calculated from charms) if assetData has 0
    const decimals = assetData?.decimals || 8;
    const assetSupply = assetData?.total_supply ? Number(assetData.total_supply) : 0;
    const holdersSupply = holdersData?.total_supply ? Number(holdersData.total_supply) : 0;
    const totalSupply = (assetSupply > 0 ? assetSupply : holdersSupply) / Math.pow(10, decimals);
    
    // Format supply with appropriate suffix
    const formatSupply = (value) => {
        if (value >= 1e9) return (value / 1e9).toFixed(2) + 'B';
        if (value >= 1e6) return (value / 1e6).toFixed(2) + 'M';
        if (value >= 1e3) return (value / 1e3).toFixed(2) + 'K';
        return value.toLocaleString(undefined, { maximumFractionDigits: 2 });
    };

    return (
        <div className="container mx-auto px-4 py-8">
            <div className="max-w-6xl mx-auto">
                {/* Breadcrumb */}
                <div className="flex items-center text-sm text-dark-400 mb-6">
                    <Link href="/" className="hover:text-primary-400">Home</Link>
                    <span className="mx-2">/</span>
                    <Link
                        href={`/?type=${asset.type}`}
                        className="hover:text-primary-400"
                    >
                        {asset.type === 'nft' ? 'NFTs' : asset.type === 'token' ? 'Tokens' : 'dApps'}
                    </Link>
                    <span className="mx-2">/</span>
                    <span className="font-medium text-white">{asset.name}</span>
                </div>

                {/* Hero Section: Title left, Image right */}
                <div className="bg-dark-800/50 rounded-xl p-6 mb-8">
                    <div className="flex flex-col lg:flex-row gap-8 items-center">
                        {/* Left: Title, Badge, Stats */}
                        <div className="flex-1 space-y-4">
                            <div className="flex items-center gap-3">
                                {isBroToken && (
                                    <span className="px-3 py-1 rounded-full text-sm font-medium bg-gradient-to-r from-amber-400 to-yellow-500 text-white">
                                        🪙 $BRO
                                    </span>
                                )}
                                <span className={`px-3 py-1 rounded-full text-xs font-medium ${
                                    asset.type === 'token' ? 'bg-blue-500/20 text-blue-400' :
                                    asset.type === 'nft' ? 'bg-purple-500/20 text-purple-400' :
                                    'bg-green-500/20 text-green-400'
                                }`}>
                                    {typeLabel}
                                </span>
                                {asset.verified && (
                                    <span className="px-2 py-1 rounded-full text-xs bg-green-500/20 text-green-400">
                                        ✓ Verified
                                    </span>
                                )}
                            </div>
                            
                            <h1 className="text-4xl font-bold text-white">{asset.name || 'Unnamed Asset'}</h1>
                            
                            {asset.ticker && (
                                <p className="text-xl text-dark-300 font-mono">${asset.ticker}</p>
                            )}
                            
                            <p className="text-dark-400 text-sm">
                                Created on {formattedDate}
                            </p>

                            {/* Token Stats */}
                            {asset.type === 'token' && (
                                <div className="grid grid-cols-2 gap-4 mt-4">
                                    <div className="bg-dark-700/50 rounded-lg p-4">
                                        <div className="text-sm text-dark-400">Total Supply</div>
                                        <div className="text-2xl font-bold text-primary-400">{formatSupply(totalSupply)}</div>
                                    </div>
                                    <div className="bg-dark-700/50 rounded-lg p-4">
                                        <div className="text-sm text-dark-400">Decimals</div>
                                        <div className="text-2xl font-bold text-white">{decimals}</div>
                                    </div>
                                </div>
                            )}
                        </div>

                        {/* Right: Image */}
                        <div className="lg:w-80 w-full">
                            <div className="aspect-square rounded-xl overflow-hidden bg-dark-700 border border-dark-600">
                                <img
                                    src={displayImage}
                                    alt={asset.name}
                                    className="w-full h-full object-cover"
                                    onError={() => setImageError(true)}
                                />
                            </div>
                        </div>
                    </div>
                </div>

                {/* Description */}
                {(asset.description || nftMetadata?.description) && (
                    <div className="mb-8">
                        <h2 className="text-xl font-semibold mb-3 text-white">Description</h2>
                        <p className="text-dark-300">{asset.description || nftMetadata?.description}</p>
                    </div>
                )}

                {/* [RJJ-STATS-HOLDERS] Tabs Navigation */}
                <div className="mb-8">
                    <div className="border-b border-dark-700">
                        <nav className="-mb-px flex space-x-8">
                            <button
                                onClick={() => setActiveTab('details')}
                                className={`py-4 px-1 border-b-2 font-medium text-sm transition-colors ${activeTab === 'details'
                                    ? 'border-primary-500 text-primary-400'
                                    : 'border-transparent text-dark-400 hover:text-white hover:border-dark-500'
                                    }`}
                            >
                                Details
                            </button>
                            <button
                                onClick={() => setActiveTab('holders')}
                                className={`py-4 px-1 border-b-2 font-medium text-sm transition-colors ${activeTab === 'holders'
                                    ? 'border-primary-500 text-primary-400'
                                    : 'border-transparent text-dark-400 hover:text-white hover:border-dark-500'
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
                                        <h2 className="text-xl font-semibold mb-3 text-white">Attributes</h2>
                                        <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
                                            {asset.attributes.map((attr, index) => (
                                                <div key={index} className="bg-dark-800 rounded-lg p-3">
                                                    <div className="text-sm text-dark-400">{attr.trait_type}</div>
                                                    <div className="font-medium text-white">{attr.value}</div>
                                                </div>
                                            ))}
                                        </div>
                                    </div>
                                )}

                                {/* dApp URL */}
                                {asset.type === 'dapp' && asset.url && (
                                    <div className="mb-8">
                                        <h2 className="text-xl font-semibold mb-3 text-white">Application</h2>
                                        <a
                                            href={asset.url}
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            className="inline-block bg-green-600 hover:bg-green-700 text-white font-medium py-2 px-4 rounded-md transition-colors"
                                        >
                                            Visit dApp
                                        </a>
                                    </div>
                                )}

                                {/* Technical details */}
                                <div className="mb-8">
                                    <h2 className="text-xl font-semibold mb-3 text-white">Technical Details</h2>
                                    <div className="bg-dark-800 rounded-lg p-4">
                                        <div className="grid grid-cols-1 gap-4">
                                            <div>
                                                <div className="text-sm text-dark-400">ID</div>
                                                <div className="font-mono text-sm break-all text-dark-200">{asset.id}</div>
                                            </div>
                                            <div>
                                                <div className="text-sm text-dark-400">Transaction</div>
                                                <Link 
                                                    href={`/tx?txid=${asset.txid}`}
                                                    className="font-mono text-sm break-all text-primary-400 hover:text-primary-300 hover:underline"
                                                >
                                                    {asset.txid}:{asset.outputIndex} →
                                                </Link>
                                            </div>
                                            {asset.utxoId && (
                                                <div>
                                                    <div className="text-sm text-dark-400">Input UTXO ID</div>
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

                                {/* Spell Metadata - Dynamic fields from spell data */}
                                {hasExtraFields && (
                                    <div className="mb-8">
                                        <h2 className="text-xl font-semibold mb-3">Spell Metadata</h2>
                                        <div className="bg-gray-100 dark:bg-gray-800 rounded-lg p-4">
                                            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                                                {Object.entries(spellMetadata.extraFields).map(([key, value]) => (
                                                    <div key={key} className="border-b border-gray-200 dark:border-gray-700 pb-3 last:border-b-0">
                                                        <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">
                                                            {formatFieldName(key)}
                                                        </div>
                                                        <div className="font-mono text-sm break-all">
                                                            {typeof value === 'string' && (value.startsWith('http://') || value.startsWith('https://')) ? (
                                                                <a 
                                                                    href={value} 
                                                                    target="_blank" 
                                                                    rel="noopener noreferrer"
                                                                    className="text-indigo-600 hover:text-indigo-800 dark:text-indigo-400 dark:hover:text-indigo-300 hover:underline"
                                                                >
                                                                    {value}
                                                                </a>
                                                            ) : (
                                                                formatFieldValue(value)
                                                            )}
                                                        </div>
                                                    </div>
                                                ))}
                                            </div>
                                        </div>
                                    </div>
                                )}
                            </div>
                        )}

                        {/* [RJJ-STATS-HOLDERS] Holders Tab */}
                        {activeTab === 'holders' && (
                            <HoldersTab appId={asset.app_id || asset.appId || asset.id} />
                        )}
                    </div>
                </div>

                {/* Back link */}
                <div className="flex items-center justify-end border-t border-dark-700 pt-6">
                    <Link
                        href={`/?type=${asset.type}`}
                        className="text-primary-400 hover:text-primary-300 transition-colors"
                    >
                        ← Back to {asset.type === 'nft' ? 'NFTs' : asset.type === 'token' ? 'Tokens' : 'dApps'}
                    </Link>
                </div>
            </div>
        </div>
    );
}
