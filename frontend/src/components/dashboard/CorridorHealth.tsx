import React from 'react';
import { Badge } from '../ui/badge';
import { Activity, ShieldCheck, AlertTriangle } from 'lucide-react';

interface Corridor {
    id: string;
    name: string;
    status: 'optimal' | 'degraded' | 'down';
    volume24h: number;
    uptime: number;
}

interface CorridorHealthProps {
    corridors: Corridor[];
}

export const CorridorHealth: React.FC<CorridorHealthProps> = ({ corridors }) => {
    const getStatusIcon = (status: string) => {
        switch (status) {
            case 'optimal': return <ShieldCheck className="w-4 h-4 text-green-500" />;
            case 'degraded': return <AlertTriangle className="w-4 h-4 text-yellow-500" />;
            case 'down': return <Activity className="w-4 h-4 text-red-500" />;
            default: return <Activity className="w-4 h-4 text-muted-foreground" />;
        }
    };

    return (
        <div className="p-6 h-full">
            <div className="flex items-center justify-between mb-6">
                <h3 className="text-sm font-bold uppercase tracking-widest text-muted-foreground">Corridor Analytics</h3>
                <Badge variant="outline" className="text-[10px] font-mono border-border/50">ACTIVE_MONITORING</Badge>
            </div>
            <div className="space-y-4">
                {corridors.map((corridor) => (
                    <div key={corridor.id} className="group p-4 rounded-xl glass border border-border/20 hover:border-accent/40 transition-all duration-300">
                        <div className="flex items-center justify-between mb-3">
                            <div className="flex items-center gap-2">
                                {getStatusIcon(corridor.status)}
                                <span className="font-bold text-sm tracking-tight">{corridor.name}</span>
                            </div>
                            <span className="text-[10px] font-mono text-muted-foreground uppercase">{corridor.uptime}% UP</span>
                        </div>

                        <div className="flex items-end justify-between">
                            <div>
                                <p className="text-[10px] uppercase tracking-widest text-muted-foreground/50 mb-1">Volume 24h</p>
                                <div className="text-sm font-mono font-bold">
                                    {new Intl.NumberFormat('en-US', {
                                        style: 'currency',
                                        currency: 'USD',
                                        notation: 'compact'
                                    }).format(corridor.volume24h)}
                                </div>
                            </div>

                            <div className={`text-[10px] font-bold px-2 py-0.5 rounded uppercase tracking-tighter ${corridor.status === 'optimal' ? 'bg-green-500/10 text-green-500' :
                                    corridor.status === 'degraded' ? 'bg-yellow-500/10 text-yellow-500' :
                                        'bg-red-500/10 text-red-500'
                                }`}>
                                {corridor.status}
                            </div>
                        </div>

                        <div className="mt-3 h-1 w-full bg-white/5 rounded-full overflow-hidden">
                            <div
                                className={`h-full transition-all duration-1000 ${corridor.status === 'optimal' ? 'bg-green-500' :
                                        corridor.status === 'degraded' ? 'bg-yellow-500' : 'bg-red-500'
                                    }`}
                                style={{ width: `${corridor.uptime}%` }}
                            />
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
};
