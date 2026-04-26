import { NextResponse } from 'next/server';\nimport { logger } from '@/lib/logger';\nimport {\n  NetworkGraphData,\n  GraphNode,\n  GraphLink,\n  validateNetworkGraphData,\n} from '@/types/network-graph';

/**
 * GET /api/network-graph
 * Returns network graph data with anchors and assets
 */
export async function GET(): Promise<NextResponse<NetworkGraphData>> {
  try {
    const backendUrl = process.env.NEXT_PUBLIC_BACKEND_URL || 'http://localhost:8080';

    // Fetch anchors and corridors from backend
    const [anchorsRes, corridorsRes] = await Promise.all([
      fetch(`${backendUrl}/api/anchors`, { cache: 'no-store' }),
      fetch(`${backendUrl}/api/corridors`, { cache: 'no-store' }),
    ]);

    if (!anchorsRes.ok || !corridorsRes.ok) {
      return NextResponse.json(
        { nodes: [], links: [] },
        { status: 500 }
      );
    }

    const anchors = await anchorsRes.json();
    const corridors = await corridorsRes.json();

    // Build nodes map for efficient lookup
    const nodesMap = new Map<string, GraphNode>();
    const links: GraphLink[] = [];

    interface AnchorRecord {
      id?: string;
      name?: string;
      reliability_score?: number;
      address?: string;
      status?: string;
    }
    interface CorridorRecord {
      source_anchor_id?: string;
      destination_anchor_id?: string;
      source_asset_code?: string;
      source_asset_issuer?: string;
      volume_usd?: number;
      success_rate?: number;
      liquidity_score?: number;
    }

    // Process anchors into nodes
    if (Array.isArray(anchors)) {
      anchors.forEach((anchor: AnchorRecord) => {
        if (anchor.id && anchor.name) {
          nodesMap.set(anchor.id, {
            id: anchor.id,
            name: anchor.name,
            type: 'anchor',
            val: anchor.reliability_score || 50,
            address: anchor.address,
            status: anchor.status,
          });
        }
      });
    }

    // Process corridors into links and asset nodes
    if (Array.isArray(corridors)) {
      corridors.forEach((corridor: CorridorRecord) => {
        if (corridor.source_anchor_id && corridor.destination_anchor_id) {
          // Create asset node if it doesn't exist
          const assetId = `${corridor.source_asset_code}_${corridor.source_asset_issuer}`;
          if (!nodesMap.has(assetId)) {
            nodesMap.set(assetId, {
              id: assetId,
              name: corridor.source_asset_code || 'Unknown',
              type: 'asset',
              val: 30,
              fullName: corridor.source_asset_code,
              issuer: corridor.source_asset_issuer,
            });
          }

          // Create links: source anchor -> asset -> destination anchor
          links.push({
            source: corridor.source_anchor_id,
            target: assetId,
            type: 'issuance',
            value: corridor.volume_usd || 0,
          });

          links.push({
            source: assetId,
            target: corridor.destination_anchor_id,
            type: 'corridor',
            value: corridor.volume_usd || 0,
            health: corridor.success_rate,
            liquidity: corridor.liquidity_score,
          });
        }
      });
    }

    // Convert map to array
    const nodes: GraphNode[] = Array.from(nodesMap.values());

    const graphData: NetworkGraphData = {
      nodes,
      links,
    };

    // Validate data before returning
    if (!validateNetworkGraphData(graphData)) {
      logger.error('Generated invalid network graph data');
      return NextResponse.json(
        { nodes: [], links: [] },
        { status: 500 }
      );
    }

    return NextResponse.json(graphData);
  } catch (error) {
    logger.error('Error fetching network graph data', error);
    return NextResponse.json(
      { nodes: [], links: [] },
      { status: 500 }
    );
  }
}

