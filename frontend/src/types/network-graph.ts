// Network Graph Types for Type Safety

export interface GraphNode {
  id: string;
  name: string;
  type: 'anchor' | 'asset';
  val: number;
  // Anchor-specific fields
  address?: string;
  status?: string;
  // Asset-specific fields
  fullName?: string;
  issuer?: string;
}

export interface GraphLink {
  source: string;
  target: string;
  type: 'issuance' | 'corridor';
  value: number;
  // Corridor-specific fields
  health?: number;
  liquidity?: number;
}

export interface NetworkGraphData {
  nodes: GraphNode[];
  links: GraphLink[];
}

// Validation functions for runtime type checking
export function validateGraphNode(node: any): node is GraphNode {
  return (
    typeof node.id === 'string' &&
    typeof node.name === 'string' &&
    (node.type === 'anchor' || node.type === 'asset') &&
    typeof node.val === 'number' &&
    // Optional fields validation
    (node.address === undefined || typeof node.address === 'string') &&
    (node.status === undefined || typeof node.status === 'string') &&
    (node.fullName === undefined || typeof node.fullName === 'string') &&
    (node.issuer === undefined || typeof node.issuer === 'string')
  );
}

export function validateGraphLink(link: any): link is GraphLink {
  return (
    typeof link.source === 'string' &&
    typeof link.target === 'string' &&
    (link.type === 'issuance' || link.type === 'corridor') &&
    typeof link.value === 'number' &&
    // Optional fields validation
    (link.health === undefined || typeof link.health === 'number') &&
    (link.liquidity === undefined || typeof link.liquidity === 'number')
  );
}

export function validateNetworkGraphData(data: any): data is NetworkGraphData {
  if (!data || typeof data !== 'object') {
    return false;
  }
  
  if (!Array.isArray(data.nodes) || !Array.isArray(data.links)) {
    return false;
  }
  
  return (
    data.nodes.every(validateGraphNode) &&
    data.links.every(validateGraphLink)
  );
}