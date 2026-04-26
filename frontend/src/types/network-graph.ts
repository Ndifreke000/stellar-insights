export interface GraphNode {
    id: string;
    name: string;
    type: 'anchor' | 'asset' | 'corridor' | string;
    val: number;
    address?: string;
    status?: string;
    fullName?: string;
    issuer?: string;
    [key: string]: string | number | undefined;
}

export interface GraphLink {
    source: string;
    target: string;
    type: 'issuance' | 'corridor' | string;
    value: number;
    health?: number;
    liquidity?: number;
    [key: string]: string | number | undefined;
}

export interface NetworkGraphData {
    nodes: GraphNode[];
    links: GraphLink[];
}

export function validateNetworkGraphData(data: unknown): data is NetworkGraphData {
    if (!data || typeof data !== 'object') return false;
    const d = data as Record<string, unknown>;
    if (!Array.isArray(d.nodes) || !Array.isArray(d.links)) return false;
    return true;
}
