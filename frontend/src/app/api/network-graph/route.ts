import { NextResponse } from 'next/server';
import { NetworkGraphData, GraphNode, GraphLink, validateGraphNode, validateGraphLink } from '@/types/network-graph';

export async function GET(): Promise<NextResponse<NetworkGraphData | { error: string }>> {
  try {
    const nodes: GraphNode[] = [];
    const links: GraphLink[] = [];
    const assetNodesMap = new Map<string, GraphNode>();
    const anchorNodesMap = new Map<string, GraphNode>();

    // Mock anchor data
    const anchors = [
      { id: 'anchor-1', name: 'StellarOrg', address: 'GCKFBEIYTKP5RDHIE6JG7JMMYZ3UXKZGCBOHFCQXE4YZQZQZQZQZQZQZ', status: 'active' },
      { id: 'anchor-2', name: 'AnchorUSD', address: 'GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLEX', status: 'active' },
      { id: 'anchor-3', name: 'EuroAnchor', address: 'GDVXMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLEX', status: 'degraded' },
    ];

    // Mock asset data
    const assets = [
      { id: 'asset-1', name: 'USDC', fullName: 'USD Coin', issuer: 'anchor-2' },
      { id: 'asset-2', name: 'EURT', fullName: 'Euro Token', issuer: 'anchor-3' },
      { id: 'asset-3', name: 'XLM', fullName: 'Stellar Lumens', issuer: 'native' },
    ];

    // Create anchor nodes
    anchors.forEach(anchor => {
      const node: GraphNode = {
        id: anchor.id,
        name: anchor.name,
        type: 'anchor',
        val: Math.random() * 100 + 50, // Mock value
        address: anchor.address,
        status: anchor.status,
      };
      nodes.push(node);
      anchorNodesMap.set(anchor.id, node);
    });

    // Create asset nodes
    assets.forEach(asset => {
      const node: GraphNode = {
        id: asset.id,
        name: asset.name,
        type: 'asset',
        val: Math.random() * 80 + 20, // Mock value
        fullName: asset.fullName,
        issuer: asset.issuer,
      };
      nodes.push(node);
      assetNodesMap.set(asset.id, node);
    });

    // Create issuance links (anchor to asset)
    assets.forEach(asset => {
      if (asset.issuer !== 'native' && anchorNodesMap.has(asset.issuer)) {
        const link: GraphLink = {
          source: asset.issuer,
          target: asset.id,
          type: 'issuance',
          value: Math.random() * 50 + 10,
        };
        links.push(link);
      }
    });

    // Create corridor links (asset to asset)
    const corridorPairs = [
      ['asset-1', 'asset-2'], // USDC to EURT
      ['asset-1', 'asset-3'], // USDC to XLM
      ['asset-2', 'asset-3'], // EURT to XLM
    ];

    corridorPairs.forEach(([source, target]) => {
      const link: GraphLink = {
        source,
        target,
        type: 'corridor',
        value: Math.random() * 100 + 20,
        health: Math.random() * 40 + 60, // 60-100%
        liquidity: Math.random() * 1000000 + 500000, // 500k-1.5M
      };
      links.push(link);
    });

    // Validate all nodes and links before returning
    const invalidNodes = nodes.filter(n => !validateGraphNode(n));
    const invalidLinks = links.filter(l => !validateGraphLink(l));

    if (invalidNodes.length > 0 || invalidLinks.length > 0) {
      throw new Error(`Invalid graph data: ${invalidNodes.length} invalid nodes, ${invalidLinks.length} invalid links`);
    }

    const graphData: NetworkGraphData = { nodes, links };

    return NextResponse.json(graphData);
  } catch (error) {
    console.error('Network graph API error:', error);
    return NextResponse.json(
      { error: 'Failed to fetch network graph data' },
      { status: 500 }
    );
  }
}