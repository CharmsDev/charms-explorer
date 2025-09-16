'use client';

import CharmCard from './CharmCard';

export default function AssetGrid({ assets, isLoading }) {
    if (isLoading) {
        return (
            <div className="container mx-auto px-4 py-8">
                <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-6">
                    {[...Array(8)].map((_, index) => (
                        <div key={index} className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden shadow-sm h-full animate-pulse">
                            <div className="w-full h-48 bg-gray-200 dark:bg-gray-700"></div>
                            <div className="p-4">
                                <div className="h-6 bg-gray-200 dark:bg-gray-700 rounded w-3/4 mb-2"></div>
                                <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/2 mb-4"></div>
                                <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-full mb-2"></div>
                                <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-full mb-2"></div>
                                <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-3/4"></div>
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        );
    }

    console.log('AssetGrid received assets:', assets, 'Length:', assets?.length);
    
    if (!assets || assets.length === 0) {
        return (
            <div className="container mx-auto px-4 py-16 text-center">
                <h3 className="text-xl font-medium text-gray-700 dark:text-gray-300 mb-2">No assets found</h3>
                <p className="text-gray-500 dark:text-gray-400">Try adjusting your filters or search criteria</p>
            </div>
        );
    }

    return (
        <div className="container mx-auto px-4 py-8">
            <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-6">
                {assets.map((asset) => (
                    <CharmCard key={asset.id} charm={asset} />
                ))}
            </div>
        </div>
    );
}
