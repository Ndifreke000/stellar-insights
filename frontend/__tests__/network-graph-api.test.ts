/**
 * @jest-environment node
 */

import { GET } from '../src/app/api/network-graph/route';
import { validateNetworkGraphData } from '../src/types/network-graph';

describe('/api/network-graph', () => {
  it('should return valid network graph data', async () => {
    const response = await GET();
    const data = await response.json();

    // Check response structure
    expect(data).toHaveProperty('nodes');
    expect(data).toHaveProperty('links');
    expect(Array.isArray(data.nodes)).toBe(true);
    expect(Array.isArray(data.links)).toBe(true);

    // Validate with our type guards
    expect(validateNetworkGraphData(data)).toBe(true);

    // Check that we have some data
    expect(data.nodes.length).toBeGreaterThan(0);
    expect(data.links.length).toBeGreaterThan(0);

    // Validate node structure
    data.nodes.forEach((node: any) => {
      expect(typeof node.id).toBe('string');
      expect(typeof node.name).toBe('string');
      expect(['anchor', 'asset']).toContain(node.type);
      expect(typeof node.val).toBe('number');
    });

    // Validate link structure
    data.links.forEach((link: any) => {
      expect(typeof link.source).toBe('string');
      expect(typeof link.target).toBe('string');
      expect(['issuance', 'corridor']).toContain(link.type);
      expect(typeof link.value).toBe('number');
    });
  });

  it('should handle malformed data gracefully', () => {
    // Test validation functions with invalid data
    expect(validateNetworkGraphData(null)).toBe(false);
    expect(validateNetworkGraphData({})).toBe(false);
    expect(validateNetworkGraphData({ nodes: 'invalid' })).toBe(false);
    expect(validateNetworkGraphData({ nodes: [], links: 'invalid' })).toBe(false);
  });
});