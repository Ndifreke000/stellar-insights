import React from 'react';
import { Badge } from '@/components/ui/badge';

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

const getStatusVariant = (status: Corridor['status']): 'success' | 'warning' | 'destructive' => {
    switch (status) {
        case 'optimal':
            return 'success';
        case 'degraded':
            return 'warning';
        case 'down':
            return 'destructive';
        default:
            return 'success';
    }
};

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
                                <Badge variant={getStatusVariant(corridor.status)}>
                                    {corridor.status.charAt(0).toUpperCase() + corridor.status.slice(1)}
                                </Badge>
                                <span className="text-sm text-muted-foreground text-right w-12">{corridor.uptime}%</span>
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
};
