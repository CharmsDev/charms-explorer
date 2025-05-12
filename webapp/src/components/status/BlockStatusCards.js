'use client';

import { Badge } from '@/components/ui/Badge';

export default function BlockStatusCards({ indexerStatus, bitcoinStatus, blocksBehind, syncProgress, lastUpdated, isHovered, setIsHovered }) {
    return (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
            {/* Current Block Card */}
            <div
                className="transform transition-all duration-300 bg-gradient-to-br from-dark-800 to-dark-900 rounded-lg overflow-hidden shadow-lg hover:shadow-2xl"
                onMouseEnter={() => setIsHovered('current')}
                onMouseLeave={() => setIsHovered(null)}
            >
                <div className="relative p-6">
                    <div className="absolute top-0 right-0 w-32 h-32 bg-gradient-to-br from-blue-500/20 to-indigo-600/10 rounded-bl-full"></div>

                    <div className="flex items-start justify-between">
                        <div>
                            <h2 className="text-xl font-semibold text-white mb-1">Latest Bitcoin Block</h2>
                            <p className="text-dark-300 text-sm mb-4">Current blockchain height</p>
                        </div>
                        <div className="z-10">
                            <Badge className="bg-blue-600/20 text-blue-400 border border-blue-500/30 px-3 py-1">
                                <svg className="w-4 h-4 mr-1 inline animate-pulse" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
                                </svg>
                                Network
                            </Badge>
                        </div>
                    </div>

                    <div className="flex items-center mt-2">
                        <div className="mr-4">
                            <div className="text-5xl font-bold bg-gradient-to-r from-blue-400 to-indigo-500 bg-clip-text text-transparent">
                                {bitcoinStatus.block_count || '-'}
                            </div>
                            <div className="text-dark-400 text-sm mt-1">
                                {bitcoinStatus.network || 'Bitcoin'} Network
                            </div>
                        </div>
                        <div className="flex-grow">
                            <div className="flex justify-between text-sm text-dark-400 mb-1">
                                <span>Block Hash</span>
                            </div>
                            <div className="bg-dark-800/50 rounded-md p-2 font-mono text-xs text-dark-300 truncate" title={bitcoinStatus.best_block_hash || '-'}>
                                {bitcoinStatus.best_block_hash ?
                                    (bitcoinStatus.best_block_hash.length > 20 ?
                                        `${bitcoinStatus.best_block_hash.substring(0, 10)}...${bitcoinStatus.best_block_hash.substring(bitcoinStatus.best_block_hash.length - 10)}`
                                        : bitcoinStatus.best_block_hash)
                                    : '-'}
                            </div>
                        </div>
                    </div>

                    <div className={`absolute bottom-0 left-0 h-1 bg-gradient-to-r from-blue-400 to-indigo-500 transition-all duration-500 ${isHovered === 'current' ? 'w-full' : 'w-0'}`}></div>
                </div>
            </div>

            {/* Processed Block Card */}
            <div
                className="transform transition-all duration-300 bg-gradient-to-br from-dark-800 to-dark-900 rounded-lg overflow-hidden shadow-lg hover:shadow-2xl"
                onMouseEnter={() => setIsHovered('processed')}
                onMouseLeave={() => setIsHovered(null)}
            >
                <div className="relative p-6">
                    <div className="absolute top-0 right-0 w-32 h-32 bg-gradient-to-br from-emerald-500/20 to-teal-600/10 rounded-bl-full"></div>

                    <div className="flex items-start justify-between">
                        <div>
                            <h2 className="text-xl font-semibold text-white mb-1">Last Processed Block</h2>
                            <p className="text-dark-300 text-sm mb-4">Most recent indexed block</p>
                        </div>
                        <div className="z-10">
                            <Badge className="bg-emerald-600/20 text-emerald-400 border border-emerald-500/30 px-3 py-1">
                                <svg className="w-4 h-4 mr-1 inline animate-pulse" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z"></path>
                                </svg>
                                Indexer
                            </Badge>
                        </div>
                    </div>

                    <div className="flex items-center mt-2">
                        <div className="mr-4">
                            <div className="text-5xl font-bold bg-gradient-to-r from-emerald-400 to-teal-500 bg-clip-text text-transparent">
                                {indexerStatus.last_processed_block || '-'}
                            </div>
                            <div className="text-dark-400 text-sm mt-1">
                                {blocksBehind > 0 ? `${blocksBehind} blocks behind` : 'Fully synced'}
                            </div>
                        </div>
                        <div className="flex-grow">
                            <div className="flex justify-between text-sm text-dark-400 mb-1">
                                <span>Sync Progress</span>
                                <span>{syncProgress}%</span>
                            </div>
                            <div className="bg-dark-800/50 rounded-full h-2 overflow-hidden">
                                <div
                                    className="h-full bg-gradient-to-r from-emerald-400 to-teal-500 transition-all duration-500"
                                    style={{ width: `${syncProgress}%` }}
                                ></div>
                            </div>
                            <div className="mt-2 text-xs text-dark-400">
                                Last updated: {lastUpdated.toLocaleString()}
                            </div>
                        </div>
                    </div>

                    <div className={`absolute bottom-0 left-0 h-1 bg-gradient-to-r from-emerald-400 to-teal-500 transition-all duration-500 ${isHovered === 'processed' ? 'w-full' : 'w-0'}`}></div>
                </div>
            </div>
        </div>
    );
}
