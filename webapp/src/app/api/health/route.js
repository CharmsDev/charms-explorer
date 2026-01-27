import { NextResponse } from 'next/server';
import packageJson from '../../../../package.json';

export async function GET() {
    return NextResponse.json({
        status: 'ok',
        version: packageJson.version,
        name: packageJson.name,
        timestamp: new Date().toISOString()
    });
}
