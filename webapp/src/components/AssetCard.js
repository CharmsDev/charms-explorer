'use client';

import { useState } from 'react';
import Link from 'next/link';

export default function AssetCard({ asset }) {
    const [imageError, setImageError] = useState(false);
    const [isHovered, setIsHovered] = useState(false);
    const [imageLoaded, setImageLoaded] = useState(false);
    const placeholderImage = "/images/logo.png";

    // Format date
    const formattedDate = new Date(asset.created_at).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric'
    });

    // Get type-specific styling and icon
    const getTypeDetails = () => {
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

    // Get display name with fallback
    const getDisplayName = () => {
        if (asset.name) return asset.name;
        if (asset.symbol) return asset.symbol;
        return `Asset ${asset.id}`;
    };

    // Get display image with fallback
    const getDisplayImage = () => {
        if (imageError || !asset.image_url) return placeholderImage;
        return asset.image_url;
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
                {asset.symbol && asset.symbol !== asset.name && (
                    <p className="text-sm text-dark-400 font-mono">
                        ${asset.symbol}
                    </p>
                )}

                {/* Description */}
                {asset.description && (
                    <p className="text-sm text-dark-500 line-clamp-2">
                        {asset.description}
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
