'use client';

import { useState, useEffect, useMemo } from 'react';
import Link from 'next/link';
import { classifyCharm, getCharmBadgeProps, CHARM_TYPES } from '@/services/charmClassifier';
import { getRefNftAppId, getCachedNftReference, fetchNftReferenceMetadata } from '@/services/api/tokenMetadata';
import { getReferenceNftImage, extractHashFromAppId, fetchReferenceNftByHash } from '@/services/api/referenceNft';
import { getDisplayMetadata } from '@/services/spellParser';

export default function AssetCard({ asset, nftReferenceMap }) {
    const [imageError, setImageError] = useState(false);
    const [isHovered, setIsHovered] = useState(false);
    const [imageLoaded, setImageLoaded] = useState(false);
    const [nftMetadata, setNftMetadata] = useState(null);
    const [spellImage, setSpellImage] = useState(null);
    const placeholderImage = "/images/logo.png";

    // Fetch metadata based on asset type
    // Both tokens and NFTs can fetch image from reference NFT
    useEffect(() => {
        const appId = asset.app_id || asset.id;
        
        // For tokens (t/...) or NFTs (n/...) without a real image, fetch from reference NFT endpoint
        const hasOwnImage = asset.image && asset.image !== '/images/logo.png' && asset.image !== placeholderImage;
        if ((appId?.startsWith('t/') || appId?.startsWith('n/')) && !hasOwnImage && !asset.image_url) {
            const hash = extractHashFromAppId(appId);
            if (hash) {
                fetchReferenceNftByHash(hash).then(refNft => {
                    if (refNft?.image_url) {
                        setSpellImage(refNft.image_url);
                    }
                });
            }
        }
    }, [asset.app_id, asset.id, asset.image, asset.image_url]);

    // Classify the charm to get special badges
    const charmType = classifyCharm(asset);
    const isSpecialType = [CHARM_TYPES.BRO_TOKEN, CHARM_TYPES.CHARMS_CAST_DEX, CHARM_TYPES.DEX_ORDER].includes(charmType);

    // Format date - handle missing/invalid dates
    const formatDate = () => {
        const dateStr = asset.created_at || asset.timestamp;
        if (!dateStr) return null;
        const date = new Date(dateStr);
        if (isNaN(date.getTime())) return null;
        return date.toLocaleDateString('en-US', {
            year: 'numeric',
            month: 'short',
            day: 'numeric'
        });
    };
    const formattedDate = formatDate();

    // Get type-specific styling and icon - Dark mode cohesive palette
    const getTypeDetails = () => {
        // Special handling for BRO token (verified)
        if (charmType === CHARM_TYPES.BRO_TOKEN) {
            return {
                color: 'from-amber-400 to-yellow-500',
                bgColor: 'bg-amber-500/10',
                borderColor: 'border-amber-500/20',
                icon: '🪙',
                label: '$BRO'
            };
        }
        
        // Special handling for Charms Cast DEX
        if (charmType === CHARM_TYPES.CHARMS_CAST_DEX) {
            return {
                color: 'from-violet-400 to-purple-500',
                bgColor: 'bg-violet-500/10',
                borderColor: 'border-violet-500/20',
                icon: '🔄',
                label: 'DEX'
            };
        }

        switch (asset.asset_type) {
            case 'nft':
                return {
                    color: 'from-purple-400 to-violet-500',
                    bgColor: 'bg-purple-500/10',
                    borderColor: 'border-purple-500/20',
                    icon: '🖼️',
                    label: 'NFT'
                };
            case 'token':
                return {
                    color: 'from-amber-300 to-orange-400',
                    bgColor: 'bg-amber-500/10',
                    borderColor: 'border-amber-500/20',
                    icon: '🪙',
                    label: 'Token'
                };
            case 'dapp':
                return {
                    color: 'from-slate-400 to-slate-500',
                    bgColor: 'bg-slate-500/10',
                    borderColor: 'border-slate-500/20',
                    icon: '⚙️',
                    label: 'dApp'
                };
            default:
                return {
                    color: 'from-slate-400 to-slate-500',
                    bgColor: 'bg-slate-500/10',
                    borderColor: 'border-slate-500/20',
                    icon: '📦',
                    label: asset.asset_type || 'Asset'
                };
        }
    };

    const typeDetails = getTypeDetails();

    // Format supply for display with dynamic decimals [RJJ-DECIMALS]
    const formatSupply = (supplyRaw, decimals = 8) => {
        if (!supplyRaw) return 'N/A';

        // Convert from raw to actual value using decimals
        const supply = supplyRaw / Math.pow(10, decimals);

        // Format with appropriate suffix
        if (supply >= 1e12) return `${(supply / 1e12).toFixed(2)}T`;
        if (supply >= 1e9) return `${(supply / 1e9).toFixed(2)}B`;
        if (supply >= 1e6) return `${(supply / 1e6).toFixed(2)}M`;
        if (supply >= 1e3) return `${(supply / 1e3).toFixed(2)}K`;

        // For smaller numbers, show with appropriate decimal places
        if (supply >= 1) return supply.toLocaleString(undefined, { maximumFractionDigits: 2 });
        return supply.toFixed(decimals);
    };

    // Get normalized metadata using the standardized parser
    const displayMeta = useMemo(() => {
        return getDisplayMetadata(asset, nftMetadata);
    }, [asset, nftMetadata]);

    // Get display name with fallback
    // Assets are now pre-transformed with name field from spell metadata
    const getDisplayName = () => {
        // First check if asset already has transformed name field
        if (asset.name) return asset.name;
        if (displayMeta.name) return displayMeta.name;
        // Fallback: show shortened app_id
        const appId = asset.app_id || asset.id;
        if (appId) {
            const prefix = appId.substring(0, 2);
            const hash = appId.substring(2, 10);
            return `Charm ${prefix}${hash}`;
        }
        return `Asset ${asset.id}`;
    };

    // Get display image with fallback
    // Both tokens and NFTs can use image from reference NFT
    const getDisplayImage = () => {
        if (imageError) return placeholderImage;
        // Use spell image fetched from reference NFT (for both tokens and NFTs)
        if (spellImage) return spellImage;
        // Check asset's own image fields
        if (asset.image && asset.image !== '/images/logo.png') return asset.image;
        if (asset.image_url) return asset.image_url;
        // Try displayMeta (for raw assets)
        if (displayMeta.image) return displayMeta.image;
        return placeholderImage;
    };
    
    // Get display description
    const getDisplayDescription = () => displayMeta.description;
    
    // Get display symbol
    const getDisplaySymbol = () => displayMeta.ticker;

    const isPlaceholder = getDisplayImage() === placeholderImage;

    // Generate URLs - asset detail and transaction detail
    const assetDetailUrl = `/asset/${encodeURIComponent(asset.app_id || asset.id)}`;
    const txDetailUrl = asset.transaction_hash ? `/tx?txid=${asset.transaction_hash}` : null;

    return (
        <div
            className={`relative bg-dark-900 border ${typeDetails.borderColor} rounded-xl p-4 hover:shadow-xl transition-all duration-300 group ${typeDetails.bgColor}`}
            onMouseEnter={() => setIsHovered(true)}
            onMouseLeave={() => setIsHovered(false)}
        >
            {/* Type Badge */}
            <div className={`absolute top-3 right-3 px-2 py-1 rounded-full text-xs font-medium bg-gradient-to-r ${typeDetails.color} text-white shadow-lg z-10`}>
                <span className="mr-1">{typeDetails.icon}</span>
                {typeDetails.label}
            </div>

            {/* Asset Image - Links to asset detail */}
            <Link href={assetDetailUrl} className="block relative mb-4 cursor-pointer">
                <div className="aspect-square rounded-lg overflow-hidden bg-dark-800">
                    <img
                        src={getDisplayImage()}
                        alt={getDisplayName()}
                        className={`w-full h-full object-cover transition-all duration-300 ${!isPlaceholder ? 'group-hover:scale-105' : `opacity-60 group-hover:opacity-100`}`}
                        onError={() => {
                            if (!imageError) {
                                setImageError(true);
                            }
                        }}
                        onLoad={() => {
                            if (!imageLoaded) {
                                setImageLoaded(true);
                            }
                        }}
                    />
                </div>

                {/* Network Badge - Color coded */}
                <div className={`absolute bottom-2 left-2 px-2 py-1 backdrop-blur-sm rounded-md text-xs font-medium ${
                    asset.network === 'mainnet' 
                        ? 'bg-orange-500/20 text-orange-400 border border-orange-500/30' 
                        : 'bg-blue-500/20 text-blue-400 border border-blue-500/30'
                }`}>
                    {asset.network === 'mainnet' ? '₿ Mainnet' : '₿ Testnet4'}
                </div>
            </Link>

            {/* Asset Info */}
            <div className="space-y-2">
                {/* Type label + Name (+ Symbol if different from name) */}
                <div className="flex items-center gap-2 mb-1">
                    <span className={`text-xs font-medium px-2 py-0.5 rounded bg-gradient-to-r ${typeDetails.color} text-white`}>
                        {typeDetails.label}
                    </span>
                </div>
                <div className="flex items-center justify-between">
                    <Link href={assetDetailUrl} className="flex-1 min-w-0">
                        <h3 className="font-semibold text-white text-lg truncate hover:text-primary-400 transition-colors cursor-pointer">
                            {getDisplayName()}
                        </h3>
                    </Link>
                    {/* Only show ticker if different from name */}
                    {getDisplaySymbol() && getDisplaySymbol().toLowerCase() !== getDisplayName().toLowerCase() && (
                        <span className="ml-2 text-sm text-dark-400 font-mono flex-shrink-0">
                            ${getDisplaySymbol()}
                        </span>
                    )}
                </div>

                {/* Description */}
                {getDisplayDescription() && (
                    <p className="text-sm text-dark-500 line-clamp-2">
                        {getDisplayDescription()}
                    </p>
                )}

                {/* Supply Info */}
                {asset.total_supply && (
                    <div className="flex items-center justify-between text-sm">
                        <span className="text-dark-500">Total Supply:</span>
                        <span className="text-white font-medium">
                            {formatSupply(asset.total_supply, asset.decimals || 8)}
                        </span>
                    </div>
                )}

                {/* Date and Block Info */}
                <div className="flex items-center justify-between text-xs text-dark-500 pt-2 border-t border-dark-800">
                    {formattedDate && <span>{formattedDate}</span>}
                    {asset.block_height && (
                        <span>Block #{asset.block_height.toLocaleString()}</span>
                    )}
                </div>

                {/* App ID (shortened) */}
                {asset.app_id && (
                    <div className="mt-2 text-xs text-dark-500 font-mono truncate hover:text-primary-400 transition-colors cursor-pointer"
                        title={asset.app_id}>
                        {asset.app_id.length > 20 ? `${asset.app_id.substring(0, 20)}...` : asset.app_id}
                    </div>
                )}
            </div>

            {/* Transaction Link */}
            {txDetailUrl && (
                <Link 
                    href={txDetailUrl}
                    className="mt-2 flex items-center text-xs text-dark-500 hover:text-primary-400 transition-colors"
                >
                    <span className="mr-1">📜</span>
                    <span className="font-mono truncate">TX: {asset.transaction_hash?.substring(0, 12)}...</span>
                </Link>
            )}

            {/* Hover Effect Overlay */}
            {isHovered && (
                <div className="absolute inset-0 bg-gradient-to-br from-primary-500/10 to-transparent rounded-xl pointer-events-none" />
            )}
        </div>
    );
}
