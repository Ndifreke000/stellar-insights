import React from 'react';

interface Corridor {
  id: string;
  health: number;
  successRate: number;
}

interface CorridorHealthCardProps {
  corridors: Corridor[];
}

export function CorridorHealthCard({ corridors }: CorridorHealthCardProps) {
  return (
    <div className="col-span-1 bg-white rounded shadow p-4">
      <h2 className="text-sm text-gray-500">Active Corridor Health</h2>
      <ul className="mt-3 space-y-3">
        {corridors.map((c) => (
          <li key={c.id} className="flex items-center justify-between">
            <div>
              <div className="font-medium">{c.id}</div>
              <div className="text-sm text-gray-500">
                Success: {(c.successRate * 100).toFixed(2)}%
              </div>
            </div>
            <div className="text-sm font-semibold">
              {(c.health * 100).toFixed(0)}%
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
