'use client';

import Link from 'next/link';

export default function AssetTechnicalDetails({ asset, holdersData }) {
    const nftOwner = asset.type === 'nft' && holdersData?.holders?.[0]?.address;
    
    return (
        <div className="bg-dark-800 rounded-lg p-4">
            <div className="grid grid-cols-1 gap-4">
                {/* App ID */}
                <div>
                    <div className="text-sm text-dark-400">App ID</div>
                    <div className="font-mono text-sm break-all text-dark-200">{asset.id || asset.app_id}</div>
                </div>
                
                {/* Transaction - for NFTs */}
                {asset.txid && (
                    <div>
                        <div className="text-sm text-dark-400">Transaction</div>
                        <Link 
                            href={`/tx?txid=${asset.txid}`}
                            className="font-mono text-sm break-all text-primary-400 hover:text-primary-300 hover:underline"
                        >
                            {asset.txid}:{asset.outputIndex ?? 0} →
                        </Link>
                    </div>
                )}
                
                {/* Owner - for NFTs */}
                {asset.type === 'nft' && (asset.address || nftOwner) && (
                    <div>
                        <div className="text-sm text-dark-400">Owner</div>
                        <Link 
                            href={`/address/${asset.address || nftOwner}`}
                            className="font-mono text-sm break-all text-primary-400 hover:text-primary-300 hover:underline"
                        >
                            {asset.address || nftOwner} →
                        </Link>
                    </div>
                )}
            </div>
        </div>
    );
}
