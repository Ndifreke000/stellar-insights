import { NextResponse } from 'next/server';

const BACKEND_URL = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080/api";

interface Anchor {
    id: string;
    name: string;
    stellar_account: string;
    status: string;
}

interface Corridor {
    id: string;
    source_asset: string;
    destination_asset: string;
    health_score: number;
    liquidity_depth_usd: number;
}

export async function GET() {
    try {
        // 1. Fetch Anchors and Corridors in parallel
        const [anchorsRes, corridorsRes] = await Promise.all([
            fetch(`${BACKEND_URL}/anchors`, { cache: 'no-store' }),
            fetch(`${BACKEND_URL}/corridors`, { cache: 'no-store' })
        ]);

        if (!anchorsRes.ok || !corridorsRes.ok) {
            throw new Error('Failed to fetch data from backend');
        }

        const anchorsData = await anchorsRes.json();
        const corridors: Corridor[] = await corridorsRes.json();
        const anchors: Anchor[] = anchorsData.anchors || [];

        // 2. Transform into Nodes and Links
        const nodes: any[] = [];
        const links: any[] = [];
        const assetNodesMap = new Map<string, any>();
        const anchorNodesMap = new Map<string, any>();

        // Add Anchor nodes
        anchors.forEach(anchor => {
            const anchorNode = {
                id: `anchor-${anchor.id}`,
                name: anchor.name,
                type: 'anchor',
                address: anchor.stellar_account,
                status: anchor.status,
                val: 15 // radius/weight
            };
            nodes.push(anchorNode);
            anchorNodesMap.set(anchor.stellar_account, anchorNode);
        });

        // Extract Asset nodes from corridors
        corridors.forEach(corridor => {
            [corridor.source_asset, corridor.destination_asset].forEach(assetStr => {
                const [code, issuer] = assetStr.split(':');
                const assetId = `asset-${assetStr}`;

                if (!assetNodesMap.has(assetId)) {
                    const assetNode = {
                        id: assetId,
                        name: code,
                        fullName: assetStr,
                        type: 'asset',
                        issuer: issuer,
                        val: 10
                    };
                    nodes.push(assetNode);
                    assetNodesMap.set(assetId, assetNode);

                    // Link Asset to its Anchor (Issuer)
                    if (issuer && issuer !== 'native' && anchorNodesMap.has(issuer)) {
                        links.push({
                            source: `anchor-${issuer}`, // Should ideally match anchor id if possible, but address is more reliable for mapping
                            target: assetId,
                            type: 'issuance',
                            value: 1
                        });
                    }
                }
            });

            // Add Corridor link
            links.push({
                source: `asset-${corridor.source_asset}`,
                target: `asset-${corridor.destination_asset}`,
                type: 'corridor',
                health: corridor.health_score,
                liquidity: corridor.liquidity_depth_usd,
                value: Math.log10(corridor.liquidity_depth_usd + 1) + 1 // Link weight
            });
        });

        return NextResponse.json({ nodes, links });
    } catch (error) {
        console.error('Network Graph API Error:', error);
        return NextResponse.json({ error: 'Failed to fetch graph data' }, { status: 500 });
    }
}
