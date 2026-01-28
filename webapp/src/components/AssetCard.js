'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';
import { classifyCharm, getCharmBadgeProps, CHARM_TYPES } from '@/services/charmClassifier';
import { getRefNftAppId, getCachedNftReference, fetchNftReferenceMetadata } from '@/services/api/tokenMetadata';

export default function AssetCard({ asset, nftReferenceMap }) {
    const [imageError, setImageError] = useState(false);
    const [isHovered, setIsHovered] = useState(false);
    const [imageLoaded, setImageLoaded] = useState(false);
    const [nftMetadata, setNftMetadata] = useState(null);
    const placeholderImage = "/images/logo.png";

    // For tokens (t/...), fetch NFT reference metadata
    useEffect(() => {
        const appId = asset.app_id || asset.id;
        if (appId?.startsWith('t/')) {
            // First check if passed via prop (preloaded)
            if (nftReferenceMap) {
                const nftAppId = getRefNftAppId(appId);
                if (nftAppId && nftReferenceMap[nftAppId]) {
                    setNftMetadata(nftReferenceMap[nftAppId]);
                    return;
                }
            }
            // Then check cache
            const cached = getCachedNftReference(appId);
            if (cached) {
                setNftMetadata(cached);
            } else {
                // Fetch if not cached
                fetchNftReferenceMetadata(appId).then(meta => {
                    if (meta) setNftMetadata(meta);
                });
            }
        }
    }, [asset.app_id, asset.id, nftReferenceMap]);

    // Classify the charm to get special badges
    const charmType = classifyCharm(asset);
    const isSpecialType = [CHARM_TYPES.BRO_TOKEN, CHARM_TYPES.CHARMS_CAST_DEX, CHARM_TYPES.DEX_ORDER].includes(charmType);

    // Format date
    const formattedDate = new Date(asset.created_at).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric'
    });

    // Get type-specific styling and icon
    const getTypeDetails = () => {
        // Special handling for BRO token
        if (charmType === CHARM_TYPES.BRO_TOKEN) {
            return {
                color: 'from-yellow-500 to-orange-600',
                bgColor: 'bg-yellow-900/20',
                borderColor: 'border-yellow-500/30',
                icon: '🪙',
                label: '$BRO'
            };
        }
        
        // Special handling for Charms Cast DEX
        if (charmType === CHARM_TYPES.CHARMS_CAST_DEX) {
            return {
                color: 'from-purple-500 to-pink-600',
                bgColor: 'bg-purple-900/20',
                borderColor: 'border-purple-500/30',
                icon: '🔄',
                label: 'DEX'
            };
        }

        switch (asset.asset_type) {
            case 'nft':
                return {
                    color: 'from-purple-500 to-indigo-700',
                    bgColor: 'bg-purple-900/20',
                    borderColor: 'border-purple-500/30',
                    icon: '🖼️',
                    label: 'NFT'
                };
            case 'token':
                return {
                    color: 'from-green-500 to-emerald-700',
                    bgColor: 'bg-green-900/20',
                    borderColor: 'border-green-500/30',
                    icon: '🪙',
                    label: 'Token'
                };
            case 'dapp':
                return {
                    color: 'from-blue-500 to-cyan-700',
                    bgColor: 'bg-blue-900/20',
                    borderColor: 'border-blue-500/30',
                    icon: '⚡',
                    label: 'DApp'
                };
            default:
                return {
                    color: 'from-gray-500 to-slate-700',
                    bgColor: 'bg-gray-900/20',
                    borderColor: 'border-gray-500/30',
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

    // Get display name with fallback - use NFT reference for tokens
    const getDisplayName = () => {
        // For tokens, prefer NFT reference metadata
        if (nftMetadata?.name) return nftMetadata.name;
        if (nftMetadata?.symbol) return nftMetadata.symbol;
        if (asset.name) return asset.name;
        if (asset.symbol) return asset.symbol;
        // Fallback: show shortened app_id
        const appId = asset.app_id || asset.id;
        if (appId) {
            const prefix = appId.substring(0, 2);
            const hash = appId.substring(2, 10);
            return `Charm ${prefix}${hash}`;
        }
        return `Asset ${asset.id}`;
    };

    // Get display image with fallback - use NFT reference for tokens
    const getDisplayImage = () => {
        if (imageError) return placeholderImage;
        // For tokens, prefer NFT reference image
        if (nftMetadata?.image_url) return nftMetadata.image_url;
        if (asset.image_url) return asset.image_url;
        return placeholderImage;
    };
    
    // Get display description - use NFT reference for tokens
    const getDisplayDescription = () => {
        if (nftMetadata?.description) return nftMetadata.description;
        return asset.description;
    };
    
    // Get display symbol - use NFT reference for tokens
    const getDisplaySymbol = () => {
        if (nftMetadata?.symbol) return nftMetadata.symbol;
        return asset.symbol;
    };

    const isPlaceholder = getDisplayImage() === placeholderImage;

    // Generate detail page URL using app_id or id
    const detailUrl = `/asset/${encodeURIComponent(asset.app_id || asset.id)}`;

    return (
        <Link href={detailUrl} className="block">
            <div
                className={`relative bg-dark-900 border ${typeDetails.borderColor} rounded-xl p-4 hover:shadow-xl transition-all duration-300 cursor-pointer group ${typeDetails.bgColor}`}
                onMouseEnter={() => setIsHovered(true)}
                onMouseLeave={() => setIsHovered(false)}
            >
            {/* Type Badge */}
            <div className={`absolute top-3 right-3 px-2 py-1 rounded-full text-xs font-medium bg-gradient-to-r ${typeDetails.color} text-white shadow-lg`}>
                <span className="mr-1">{typeDetails.icon}</span>
                {typeDetails.label}
            </div>

            {/* Asset Image */}
            <div className="relative mb-4">
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

                {/* Network Badge */}
                <div className="absolute bottom-2 left-2 px-2 py-1 bg-dark-800/90 backdrop-blur-sm rounded-md text-xs text-dark-300">
                    {asset.network}
                </div>
            </div>

            {/* Asset Info */}
            <div className="space-y-2">
                {/* Name */}
                <h3 className="font-semibold text-white text-lg truncate group-hover:text-primary-400 transition-colors">
                    {getDisplayName()}
                </h3>

                {/* Symbol */}
                {getDisplaySymbol() && getDisplaySymbol() !== getDisplayName() && (
                    <p className="text-sm text-dark-400 font-mono">
                        ${getDisplaySymbol()}
                    </p>
                )}

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
                    <span>{formattedDate}</span>
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

            {/* Hover Effect Overlay */}
            {isHovered && (
                <div className="absolute inset-0 bg-gradient-to-br from-primary-500/10 to-transparent rounded-xl pointer-events-none" />
            )}
        </div>
        </Link>
    );
}
