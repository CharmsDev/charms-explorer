'use client';

export default function NetworkSelector({ activeNetwork, setActiveNetwork }) {
    return (
        <div className="flex items-center space-x-2 bg-dark-800/50 rounded-lg p-2">
            <button
                onClick={() => setActiveNetwork('all')}
                className={`px-3 py-1.5 rounded-md text-sm font-medium transition-all duration-200 ${activeNetwork === 'all'
                        ? 'bg-primary-500 text-white'
                        : 'bg-dark-700/50 text-dark-300 hover:bg-dark-700 hover:text-white'
                    }`}
            >
                All Networks
            </button>
            <button
                onClick={() => setActiveNetwork('testnet4')}
                className={`px-3 py-1.5 rounded-md text-sm font-medium transition-all duration-200 ${activeNetwork === 'testnet4'
                        ? 'bg-blue-500 text-white'
                        : 'bg-dark-700/50 text-dark-300 hover:bg-dark-700 hover:text-white'
                    }`}
            >
                Testnet 4
            </button>
            <button
                onClick={() => setActiveNetwork('mainnet')}
                className={`px-3 py-1.5 rounded-md text-sm font-medium transition-all duration-200 ${activeNetwork === 'mainnet'
                        ? 'bg-orange-500 text-white'
                        : 'bg-dark-700/50 text-dark-300 hover:bg-dark-700 hover:text-white'
                    }`}
            >
                Mainnet
            </button>
        </div>
    );
}
