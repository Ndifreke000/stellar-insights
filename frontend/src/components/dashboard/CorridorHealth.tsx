import React from 'react';
import { Badge } from '@/components/ui/badge'; // Assuming you have a Badge component, or I will use standard tailwind

// Defines the shape of a corridor object
interface Corridor {
    id: string;
    name: string;
    status: 'optimal' | 'degraded' | 'down';
    uptime: number;
    volume24h: number;
}

interface CorridorHealthProps {
    corridors: Corridor[];
}

export const CorridorHealth: React.FC<CorridorHealthProps> = ({ corridors }) => {
    return (
        <div className="bg-card text-card-foreground rounded-xl border shadow-sm col-span-1">
            <div className="flex flex-col space-y-1.5 p-6">
                <h3 className="font-semibold leading-none tracking-tight">Corridor Health</h3>
                <p className="text-sm text-muted-foreground">Real-time status of key payment corridors.</p>
            </div>
            <div className="p-6 pt-0">
                <div className="space-y-4">
                    {corridors.map((corridor) => (
                        <div key={corridor.id} className="flex items-center justify-between">
                            <div className="flex items-center space-x-4">
                                <div className="space-y-1">
                                    <p className="text-sm font-medium leading-none">{corridor.name}</p>
                                    <p className="text-sm text-muted-foreground">Vol: ${(corridor.volume24h / 1000).toFixed(1)}k</p>
                                </div>
                            </div>
                            <div className="flex items-center space-x-2">
                                <span className={`inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ${corridor.status === 'optimal'
                                        ? 'border-transparent bg-green-500/15 text-green-600 hover:bg-green-500/25'
                                        : corridor.status === 'degraded'
                                            ? 'border-transparent bg-yellow-500/15 text-yellow-600 hover:bg-yellow-500/25'
                                            : 'border-transparent bg-red-500/15 text-red-600 hover:bg-red-500/25'
                                    }`}>
                                    {corridor.status.charAt(0).toUpperCase() + corridor.status.slice(1)}
                                </span>
                                <span className="text-sm text-muted-foreground text-right w-12">{corridor.uptime}%</span>
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
};
