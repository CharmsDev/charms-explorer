'use client';

import Link from 'next/link';
import { classifyCharm, CHARM_TYPES } from '../../services/charmClassifier';

const formatSupply = (value) => {
    if (value >= 1e9) return (value / 1e9).toFixed(2) + 'B';
    if (value >= 1e6) return (value / 1e6).toFixed(2) + 'M';
    if (value >= 1e3) return (value / 1e3).toFixed(2) + 'K';
    return value.toLocaleString(undefined, { maximumFractionDigits: 2 });
};

export default function AssetHero({ 
    asset, 
    displayImage, 
    totalSupply, 
    decimals, 
    formattedDate,
    onImageError,
    description,
    imageLoading = false
}) {
    const typeLabel = asset.type === 'nft' ? 'NFT' : asset.type === 'token' ? 'Token' : 'dApp';
    const charmType = classifyCharm(asset);
    const isBroToken = charmType === CHARM_TYPES.BRO_TOKEN;

    return (
        <div className="bg-dark-800/50 rounded-xl p-6 mb-8">
            <div className="flex flex-col lg:flex-row gap-8 items-center">
                {/* Left: Title, Badge, Stats */}
                <div className="flex-1 space-y-4">
                    <div className="flex items-center gap-3 flex-wrap">
                        {/* Network Badge - Prominent */}
                        <span className={`px-3 py-1 rounded-full text-xs font-medium ${
                            asset.network === 'mainnet' 
                                ? 'bg-orange-500/20 text-orange-400 border border-orange-500/30' 
                                : 'bg-blue-500/20 text-blue-400 border border-blue-500/30'
                        }`}>
                            ₿ {asset.network === 'mainnet' ? 'Mainnet' : 'Testnet4'}
                        </span>
                        {isBroToken && (
                            <span className="px-3 py-1 rounded-full text-sm font-medium bg-gradient-to-r from-amber-400 to-yellow-500 text-white">
                                🪙 $BRO
                            </span>
                        )}
                        <span className={`px-3 py-1 rounded-full text-xs font-medium ${
                            asset.type === 'token' ? 'bg-amber-500/20 text-amber-400' :
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
                    
                    <p className="text-dark-400 text-sm">Created on {formattedDate}</p>

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

                    {/* Description */}
                    {description && (
                        <p className="text-dark-300 mt-2">{description}</p>
                    )}
                </div>

                {/* Right: Image */}
                <div className="lg:w-80 w-full">
                    <div className="aspect-square rounded-xl overflow-hidden bg-dark-700 border border-dark-600 relative">
                        {imageLoading ? (
                            <div className="w-full h-full flex items-center justify-center">
                                <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-primary-500"></div>
                            </div>
                        ) : (
                            <img
                                src={displayImage}
                                alt={asset.name}
                                className="w-full h-full object-cover"
                                onError={onImageError}
                            />
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}
