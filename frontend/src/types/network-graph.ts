export interface GraphNode {
    id: string;
    name: string;
    type: 'anchor' | 'asset' | 'corridor' | string;
    val: number;
    address?: string;
    status?: string;
    fullName?: string;
    issuer?: string;
    [key: string]: any;
}

export interface GraphLink {
    source: string;
    target: string;
    type: 'issuance' | 'corridor' | string;
    value: number;
    health?: number;
    liquidity?: number;
    [key: string]: any;
}

export interface NetworkGraphData {
    nodes: GraphNode[];
    links: GraphLink[];
}

export function validateNetworkGraphData(data: any): data is NetworkGraphData {
    if (!data || typeof data !== 'object') return false;
    if (!Array.isArray(data.nodes) || !Array.isArray(data.links)) return false;
    return true;
}
