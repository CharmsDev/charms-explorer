'use client';

import Link from 'next/link';

export default function TokenReferenceNFT({ nftMetadata }) {
    if (!nftMetadata) return null;

    return (
        <div className="bg-gradient-to-r from-purple-900/30 to-indigo-900/30 border border-purple-500/30 rounded-xl p-6 mb-8">
            <div className="flex items-center justify-between flex-wrap gap-4">
                <div>
                    <h3 className="text-lg font-semibold text-purple-300 mb-1">Reference NFT</h3>
                    <p className="text-dark-400 text-sm">This token is controlled by a Reference NFT that defines its metadata and rules.</p>
                </div>
                <Link
                    href={`/asset/${encodeURIComponent(nftMetadata.app_id)}`}
                    className="flex items-center gap-3 bg-purple-600/20 hover:bg-purple-600/40 border border-purple-500/50 rounded-lg px-4 py-3 transition-colors"
                >
                    <div className="w-12 h-12 rounded-lg overflow-hidden bg-dark-700">
                        <img src={nftMetadata.image_url || '/images/logo.png'} alt="Reference NFT" className="w-full h-full object-cover" />
                    </div>
                    <div>
                        <div className="font-medium text-white">{nftMetadata.name || 'Reference NFT'}</div>
                        <div className="text-xs text-purple-300">View NFT Details →</div>
                    </div>
                </Link>
            </div>
        </div>
    );
}
