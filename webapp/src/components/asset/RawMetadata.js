'use client';

import { getDisplayMetadata, formatFieldName } from '../../services/spellParser';

export default function RawMetadata({ asset, nftMetadata, spellMetadata }) {
    // Use spell metadata if available, otherwise try to get display metadata
    const metadata = spellMetadata || getDisplayMetadata(asset, nftMetadata);
    
    // Standard fields to display prominently
    const standardFields = [
        { key: 'name', label: 'Name', value: metadata.name },
        { key: 'ticker', label: 'Ticker', value: metadata.ticker },
        { key: 'description', label: 'Description', value: metadata.description },
        { key: 'decimals', label: 'Decimals', value: metadata.decimals },
        { key: 'supply_limit', label: 'Supply Limit', value: metadata.supply_limit },
        { key: 'url', label: 'URL', value: metadata.url },
    ].filter(f => f.value !== null && f.value !== undefined);
    
    // Extra fields from spell
    const extraFields = Object.entries(metadata.extraFields || {});
    
    const hasData = standardFields.length > 0 || extraFields.length > 0;
    
    if (!hasData) {
        return (
            <div className="text-dark-400 text-sm py-8 text-center">
                No spell metadata available for this asset.
                <p className="text-xs mt-2 text-dark-500">
                    Metadata is extracted from the spell transaction associated with this charm.
                </p>
            </div>
        );
    }

    const renderValue = (value) => {
        if (value === null || value === undefined) return <span className="text-dark-500">-</span>;
        if (typeof value === 'boolean') return <span className="text-blue-400">{value ? 'Yes' : 'No'}</span>;
        if (typeof value === 'number') return <span className="text-green-400">{value.toLocaleString()}</span>;
        if (typeof value === 'string') {
            if (value.startsWith('http')) {
                return <a href={value} target="_blank" rel="noopener noreferrer" className="text-primary-400 hover:underline break-all">{value}</a>;
            }
            return <span className="text-white break-all">{value}</span>;
        }
        if (typeof value === 'object') {
            return <span className="text-dark-300 font-mono text-xs">{JSON.stringify(value)}</span>;
        }
        return <span>{String(value)}</span>;
    };

    return (
        <div className="space-y-6">
            {/* Standard Fields */}
            {standardFields.length > 0 && (
                <div className="bg-dark-800 rounded-lg p-4">
                    <h3 className="text-sm font-medium text-dark-400 mb-3">Spell Metadata</h3>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                        {standardFields.map(({ key, label, value }) => (
                            <div key={key} className={key === 'description' ? 'md:col-span-2' : ''}>
                                <div className="text-xs text-dark-500 mb-1">{label}</div>
                                <div className="text-sm">{renderValue(value)}</div>
                            </div>
                        ))}
                    </div>
                </div>
            )}
            
            {/* Extra Fields */}
            {extraFields.length > 0 && (
                <div className="bg-dark-800 rounded-lg p-4">
                    <h3 className="text-sm font-medium text-dark-400 mb-3">Additional Fields</h3>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                        {extraFields.map(([key, value]) => (
                            <div key={key}>
                                <div className="text-xs text-dark-500 mb-1">{formatFieldName(key)}</div>
                                <div className="text-sm">{renderValue(value)}</div>
                            </div>
                        ))}
                    </div>
                </div>
            )}
            
            {/* Raw data link */}
            {metadata.raw && (
                <details className="text-xs">
                    <summary className="text-dark-500 hover:text-dark-400 cursor-pointer">
                        View raw spell data
                    </summary>
                    <pre className="mt-2 bg-dark-900 p-3 rounded overflow-x-auto text-dark-400 font-mono">
                        {JSON.stringify(metadata.raw, null, 2)}
                    </pre>
                </details>
            )}
        </div>
    );
}
