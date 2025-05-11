'use client';

export default function BlockchainVisualization({ indexerStatus, charmStats }) {
    return (
        <div className="mb-8 bg-dark-900/50 rounded-lg p-6 shadow-lg">
            <div className="blockchain-wrapper overflow-x-auto">
                <div className="blockchain-blocks flex space-x-4 pb-4 min-w-max">
                    {/* Blockchain Blocks - Generate 6 blocks */}
                    {Array.from({ length: 6 }).map((_, index) => {
                        const blockHeight = indexerStatus.last_processed_block ? (indexerStatus.last_processed_block - (index + 1)) : '?';
                        const opacity = 1 - (index * 0.15);
                        return (
                            <div key={index} className="bitcoin-block text-center relative"
                                style={{
                                    width: '180px',
                                    minWidth: '180px',
                                    height: '180px',
                                    background: 'linear-gradient(135deg, #1e293b 0%, #0f172a 100%)',
                                    borderRadius: '8px',
                                    transform: `perspective(800px) rotateY(-5deg) rotateX(5deg) translateZ(-${index * 10}px)`,
                                    transformStyle: 'preserve-3d',
                                    boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.3), 0 4px 6px -2px rgba(0, 0, 0, 0.2)',
                                    transition: 'all 0.3s ease-in-out',
                                    opacity: opacity
                                }}>
                                <div className="absolute inset-0 bg-gradient-to-br from-blue-500/20 to-indigo-600/10 rounded-lg" style={{ opacity: opacity }}></div>
                                <div className="absolute inset-0 border border-blue-500/20 rounded-lg"></div>
                                <div className="block-body p-4 text-white relative h-full flex flex-col justify-between">
                                    <div>
                                        <div className="text-xs text-blue-400 mb-1">CONFIRMED</div>
                                        <div className="text-3xl font-bold text-white mb-2">
                                            {blockHeight}
                                        </div>
                                    </div>
                                    <div>
                                        <div className="bg-dark-800/50 rounded-md p-2 mb-2">
                                            <div className="text-sm text-dark-300 mb-1">
                                                Charms
                                            </div>
                                            <div className="text-lg font-bold text-blue-400">
                                                {/* TODO: Update API to provide charms per block */}
                                                <span title="Placeholder data - API update needed">
                                                    {Math.floor(Math.random() * 5)} <small className="text-xs text-dark-400">(demo)</small>
                                                </span>
                                            </div>
                                        </div>
                                        <div className="text-xs text-dark-500 mt-2">
                                            {/* TODO: Add actual indexing time from API */}
                                            <span title="Placeholder - API update needed">
                                                Indexed: {new Date(Date.now() - (index + 1) * 600000).toLocaleTimeString()}
                                            </span>
                                        </div>
                                    </div>
                                    <span className="absolute top-2 right-2">
                                        <svg className="w-6 h-6 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                                        </svg>
                                    </span>
                                </div>
                            </div>
                        );
                    })}
                </div>
            </div>
        </div>
    );
}
