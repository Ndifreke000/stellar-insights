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
    <section 
      className="col-span-1 bg-white rounded shadow p-4"
      aria-labelledby="corridor-health-heading"
    >
      <h2 id="corridor-health-heading" className="text-sm text-gray-500">
        Active Corridor Health
      </h2>
      <ul className="mt-3 space-y-3" role="list">
        {corridors.map((c) => (
          <li key={c.id} className="flex items-center justify-between">
            <div>
              <div className="font-medium">{c.id}</div>
              <div className="text-sm text-gray-500">
                <span className="sr-only">Success rate: </span>
                Success: {(c.successRate * 100).toFixed(2)}%
              </div>
            </div>
            <div 
              className="text-sm font-semibold"
              role="status"
              aria-label={`Health score: ${(c.health * 100).toFixed(0)}%`}
            >
              {(c.health * 100).toFixed(0)}%
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}
