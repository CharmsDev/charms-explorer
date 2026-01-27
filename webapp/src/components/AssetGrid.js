'use client';

import AssetCard from './AssetCard';

export default function AssetGrid({ assets, isLoading }) {
    if (isLoading) {
        return (
            <div className="container mx-auto px-4 py-8">
                <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-6">
                    {[...Array(8)].map((_, index) => (
                        <div key={index} className="relative bg-dark-900 border border-gray-500/30 rounded-xl p-4 animate-pulse">
                            {/* Type badge skeleton */}
                            <div className="absolute top-3 right-3 w-16 h-6 bg-gray-700 rounded-full"></div>

                            {/* Image skeleton */}
                            <div className="aspect-square rounded-lg bg-dark-800 mb-4"></div>

                            {/* Content skeleton */}
                            <div className="space-y-2">
                                <div className="h-6 bg-gray-700 rounded w-3/4"></div>
                                <div className="h-4 bg-gray-700 rounded w-1/2"></div>
                                <div className="h-4 bg-gray-700 rounded w-full"></div>
                                <div className="h-4 bg-gray-700 rounded w-2/3"></div>
                                <div className="flex justify-between pt-2 border-t border-dark-800">
                                    <div className="h-3 bg-gray-700 rounded w-20"></div>
                                    <div className="h-3 bg-gray-700 rounded w-16"></div>
                                </div>
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        );
    }

    if (!assets || assets.length === 0) {
        return (
            <div className="container mx-auto px-4 py-16 text-center">
                <h3 className="text-xl font-medium text-gray-700 dark:text-gray-300 mb-2">No assets found</h3>
                <p className="text-gray-500 dark:text-gray-400">Try adjusting your filters or search criteria</p>
            </div>
        );
    }

    return (
        <div className="container mx-auto px-4 pb-8">
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5">
                {assets.map((asset) => (
                    <AssetCard key={asset.id} asset={asset} />
                ))}
            </div>
        </div>
    );
}
