'use client';

import { Button } from '@/components/ui/Button';
import { API_BASE_URL } from '@/services/apiConfig';

export default function ErrorState({ error, fetchData }) {
    return (
        <div className="container mx-auto px-4 py-8">
            <h1 className="text-3xl font-bold mb-6">Indexer Status</h1>
            <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded relative" role="alert">
                <strong className="font-bold">Error: </strong>
                <span className="block sm:inline">{error}</span>
                <p className="mt-2">
                    Could not connect to the indexer. Please make sure the indexer is running and accessible at {API_BASE_URL}.
                </p>
                <Button onClick={fetchData} className="mt-4">
                    Try Again
                </Button>
            </div>
        </div>
    );
}
