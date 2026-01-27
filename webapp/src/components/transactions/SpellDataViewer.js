'use client';

/**
 * Spell Data Viewer Component
 * Displays spell data in a formatted, readable JSON view
 * Converts byte arrays to hex strings for readability
 */

import { useState } from 'react';

const formatSpellData = (data) => {
    if (!data) return '';
    
    const replacer = (key, value) => {
        // Handle byte arrays - convert to compact hex
        if (Array.isArray(value) && value.length > 4 && value.every(v => typeof v === 'number' && v >= 0 && v <= 255)) {
            const hex = value.map(b => b.toString(16).padStart(2, '0')).join('');
            return `[hex:${hex}]`;
        }
        return value;
    };
    
    return JSON.stringify(data, replacer, 2);
};

export default function SpellDataViewer({ data, title = 'Spell Data', maxHeight = '600px' }) {
    const [expanded, setExpanded] = useState(true);
    const [copied, setCopied] = useState(false);
    
    if (!data) return null;
    
    const formattedData = formatSpellData(data);
    
    const handleCopy = () => {
        navigator.clipboard.writeText(JSON.stringify(data, null, 2));
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    };
    
    return (
        <div className="bg-dark-800/50 rounded-lg border border-dark-700 overflow-hidden">
            {/* Header */}
            <div className="flex items-center justify-between px-4 py-3 border-b border-dark-700 bg-dark-900/50">
                <div className="flex items-center gap-2">
                    <button
                        onClick={() => setExpanded(!expanded)}
                        className="text-dark-400 hover:text-white transition-colors"
                    >
                        <svg 
                            className={`w-4 h-4 transition-transform ${expanded ? 'rotate-90' : ''}`} 
                            fill="none" 
                            stroke="currentColor" 
                            viewBox="0 0 24 24"
                        >
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                        </svg>
                    </button>
                    <h3 className="text-white font-semibold">{title}</h3>
                    <span className="text-xs text-dark-500">(Raw JSON)</span>
                </div>
                
                <button
                    onClick={handleCopy}
                    className="flex items-center gap-1 px-2 py-1 text-xs text-dark-400 hover:text-white hover:bg-dark-700 rounded transition-colors"
                >
                    {copied ? (
                        <>
                            <svg className="w-3 h-3 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                            </svg>
                            <span className="text-green-400">Copied!</span>
                        </>
                    ) : (
                        <>
                            <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                            </svg>
                            <span>Copy</span>
                        </>
                    )}
                </button>
            </div>
            
            {/* Content */}
            {expanded && (
                <div className="p-4">
                    <p className="text-dark-500 text-xs mb-3">
                        Byte arrays are displayed as hex strings for readability
                    </p>
                    <div 
                        className="bg-dark-900/50 rounded-lg p-4 overflow-x-auto overflow-y-auto custom-scrollbar"
                        style={{ maxHeight }}
                    >
                        <pre className="text-xs sm:text-sm text-green-400 font-mono whitespace-pre-wrap break-words">
                            {formattedData}
                        </pre>
                    </div>
                </div>
            )}
        </div>
    );
}
