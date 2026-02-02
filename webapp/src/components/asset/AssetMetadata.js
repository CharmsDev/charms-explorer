'use client';

import { formatFieldName, formatFieldValue } from '../../services/spellParser';

async function attemptHashVerification(imageUrl) {
    try {
        await fetch(imageUrl, { cache: 'no-store', mode: 'no-cors' });
        return {
            status: 'cors-error',
            message: 'Cannot verify hash: Cross-origin resource sharing (CORS) restriction',
        };
    } catch (error) {
        return { status: 'fetch-error', message: 'Cannot fetch image for verification' };
    }
}

function HashVerificationBadge({ status }) {
    const badges = {
        'pending': <span className="px-2 py-1 text-xs bg-gray-700 text-gray-300 rounded">Pending verification...</span>,
        'verifying': <span className="px-2 py-1 text-xs bg-blue-900 text-blue-300 rounded">Verifying...</span>,
        'verified': (
            <span className="px-2 py-1 text-xs bg-green-900 text-green-300 rounded flex items-center">
                <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M5 13l4 4L19 7" />
                </svg>
                Verified
            </span>
        ),
        'failed': (
            <span className="px-2 py-1 text-xs bg-red-900 text-red-300 rounded flex items-center">
                <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
                Hash mismatch
            </span>
        ),
        'error': <span className="px-2 py-1 text-xs bg-yellow-900 text-yellow-300 rounded">Verification error</span>,
    };
    return badges[status] || null;
}

export default function AssetMetadata({ 
    asset, 
    hashVerification, 
    setHashVerification, 
    imageError,
    spellMetadata 
}) {
    const hasImageHash = asset.imageHash || asset.appData;
    const hasExtraFields = spellMetadata?.extraFields && Object.keys(spellMetadata.extraFields).length > 0;

    if (!hasImageHash && !hasExtraFields) return null;

    const handleVerify = async () => {
        if (asset?.imageHash && asset?.image && !imageError) {
            setHashVerification({ status: 'verifying', computedHash: null, message: null });
            const result = await attemptHashVerification(asset.image);
            setHashVerification({ status: result.status, computedHash: null, message: result.message });
        }
    };

    return (
        <>
            {/* Hash & App Data */}
            {hasImageHash && (
                <div className="mb-8">
                    <h2 className="text-xl font-semibold mb-3 text-white">Additional Metadata</h2>
                    <div className="bg-dark-800 rounded-lg p-4">
                        <div className="grid grid-cols-1 gap-4">
                            {asset.imageHash && (
                                <div>
                                    <div className="flex items-center justify-between">
                                        <div className="text-sm text-dark-400">Image Hash</div>
                                        <HashVerificationBadge status={hashVerification.status} />
                                    </div>
                                    <div className="font-mono text-sm break-all text-dark-200">{asset.imageHash}</div>
                                    {hashVerification.message && (
                                        <div className="mt-2 text-sm text-dark-400 italic">{hashVerification.message}</div>
                                    )}
                                    <button
                                        onClick={handleVerify}
                                        className="mt-2 px-3 py-1 text-xs bg-indigo-600 hover:bg-indigo-700 text-white rounded transition-colors"
                                    >
                                        Verify Hash
                                    </button>
                                </div>
                            )}
                            {asset.appData && (
                                <div>
                                    <div className="text-sm text-dark-400">App Data</div>
                                    <div className="font-mono text-sm break-all text-dark-200">{asset.appData}</div>
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            )}

            {/* Spell Metadata - Dynamic fields */}
            {hasExtraFields && (
                <div className="mb-8">
                    <h2 className="text-xl font-semibold mb-3 text-white">Spell Metadata</h2>
                    <div className="bg-dark-800 rounded-lg p-4">
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                            {Object.entries(spellMetadata.extraFields).map(([key, value]) => (
                                <div key={key} className="border-b border-dark-700 pb-3 last:border-b-0">
                                    <div className="text-sm text-dark-400 mb-1">{formatFieldName(key)}</div>
                                    <div className="font-mono text-sm break-all text-dark-200">
                                        {typeof value === 'string' && (value.startsWith('http://') || value.startsWith('https://')) ? (
                                            <a 
                                                href={value} 
                                                target="_blank" 
                                                rel="noopener noreferrer"
                                                className="text-primary-400 hover:text-primary-300 hover:underline"
                                            >
                                                {value}
                                            </a>
                                        ) : (
                                            formatFieldValue(value)
                                        )}
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            )}
        </>
    );
}
