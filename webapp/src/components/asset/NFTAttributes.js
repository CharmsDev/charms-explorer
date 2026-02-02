'use client';

export default function NFTAttributes({ attributes }) {
    if (!attributes || attributes.length === 0) return null;

    return (
        <div className="mb-8">
            <h2 className="text-xl font-semibold mb-3 text-white">Attributes</h2>
            <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
                {attributes.map((attr, index) => (
                    <div key={index} className="bg-dark-800 rounded-lg p-3">
                        <div className="text-sm text-dark-400">{attr.trait_type}</div>
                        <div className="font-medium text-white">{attr.value}</div>
                    </div>
                ))}
            </div>
        </div>
    );
}
