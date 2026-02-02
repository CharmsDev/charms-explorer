'use client';

export default function DAppLink({ url }) {
    if (!url) return null;

    return (
        <div className="mb-8">
            <h2 className="text-xl font-semibold mb-3 text-white">Application</h2>
            <a
                href={url}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-block bg-green-600 hover:bg-green-700 text-white font-medium py-2 px-4 rounded-md transition-colors"
            >
                Visit dApp ↗
            </a>
        </div>
    );
}
