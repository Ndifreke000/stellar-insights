"use client";

import React, { useEffect, useState } from 'react';
import { NetworkGraphData, validateNetworkGraphData } from '@/types/network-graph';

interface NetworkGraphProps {
  className?: string;
}

export function NetworkGraph({ className = '' }: NetworkGraphProps) {
  const [data, setData] = useState<NetworkGraphData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchNetworkData = async () => {
      try {
        const response = await fetch('/api/network-graph');
        if (!response.ok) {
          throw new Error('Failed to fetch network graph data');
        }
        
        const result = await response.json();
        
        // Validate the response data
        if (!validateNetworkGraphData(result)) {
          throw new Error('Invalid network graph data received from API');
        }
        
        setData(result);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'An error occurred');
      } finally {
        setLoading(false);
      }
    };

    fetchNetworkData();
  }, []);

  if (loading) {
    return (
      <div className={`flex items-center justify-center p-8 ${className}`}>
        <div className="text-lg text-muted-foreground animate-pulse">
          Loading network graph...
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className={`flex items-center justify-center p-8 ${className}`}>
        <div className="text-lg text-red-500">Error: {error}</div>
      </div>
    );
  }

  if (!data) return null;

  return (
    <div className={`bg-white dark:bg-slate-800 rounded-lg shadow-md p-6 border border-gray-100 dark:border-slate-700 ${className}`}>
      <h3 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">
        Network Graph
      </h3>
      
      <div className="space-y-4">
        <div>
          <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            Nodes ({data.nodes.length})
          </h4>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
            {data.nodes.map((node) => (
              <div
                key={node.id}
                className="p-3 bg-gray-50 dark:bg-slate-700 rounded-md"
              >
                <div className="flex items-center justify-between">
                  <span className="font-medium text-sm">{node.name}</span>
                  <span className={`px-2 py-1 text-xs rounded-full ${
                    node.type === 'anchor' 
                      ? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200'
                      : 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                  }`}>
                    {node.type}
                  </span>
                </div>
                <div className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  Value: {node.val.toFixed(2)}
                </div>
                {node.status && (
                  <div className="text-xs text-gray-500 dark:text-gray-400">
                    Status: {node.status}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>

        <div>
          <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            Links ({data.links.length})
          </h4>
          <div className="space-y-2">
            {data.links.map((link, index) => (
              <div
                key={index}
                className="p-3 bg-gray-50 dark:bg-slate-700 rounded-md"
              >
                <div className="flex items-center justify-between">
                  <span className="text-sm">
                    {link.source} → {link.target}
                  </span>
                  <span className={`px-2 py-1 text-xs rounded-full ${
                    link.type === 'issuance'
                      ? 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200'
                      : 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200'
                  }`}>
                    {link.type}
                  </span>
                </div>
                <div className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  Value: {link.value.toFixed(2)}
                  {link.health && ` | Health: ${link.health.toFixed(1)}%`}
                  {link.liquidity && ` | Liquidity: ${link.liquidity.toLocaleString()}`}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}