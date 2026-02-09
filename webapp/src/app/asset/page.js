'use client';

export const runtime = 'edge';

import { useSearchParams } from 'next/navigation';
import { useRouter } from 'next/navigation';
import { useEffect } from 'react';

// Legacy redirect: /asset?appid=XXX -> /asset/XXX
export default function AssetRedirectPage() {
    const searchParams = useSearchParams();
    const router = useRouter();
    const appid = searchParams.get('appid');

    useEffect(() => {
        if (appid) {
            router.replace(`/asset/${encodeURIComponent(appid)}`);
        } else {
            router.replace('/');
        }
    }, [appid, router]);

    return (
        <div className="container mx-auto px-4 py-12 text-center">
            <p className="text-gray-500">Redirecting...</p>
        </div>
    );
}
