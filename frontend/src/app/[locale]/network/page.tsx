"use client";

import React, { useEffect, useState } from 'react';
import NetworkGraph from '@/components/charts/NetworkGraph';
import { Badge } from '@/components/ui/badge';
import { Activity, Share2, Info } from 'lucide-react';

export default function NetworkPage() {
    const [data, setData] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);

    useEffect(() => {
        async function fetchGraphData() {
            try {
                const res = await fetch('/api/network-graph');
                if (!res.ok) throw new Error('Failed to fetch graph data');
                const json = await res.json();
                setData(json);
            } catch (err) {
                setError(err.message);
            } finally {
                setLoading(false);
            }
        }
        fetchGraphData();
    }, []);

    return (
        <div className="min-h-screen p-8 lg:p-12">
            {/* Header Area */}
            <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-6 mb-12">
                <div>
                    <div className="flex items-center gap-3 mb-2">
                        <div className="p-2 bg-accent/20 rounded-lg">
                            <Share2 className="w-5 h-5 text-accent" />
                        </div>
                        <h1 className="text-4xl font-bold tracking-tighter">Network Topology</h1>
                    </div>
                    <p className="text-muted-foreground text-sm max-w-xl">
                        Visualize the complex relationships between Stellar anchors, issued assets, and payment corridors.
                        Clusters represent strong liquidity hubs and operational dependencies.
                    </p>
                </div>

                <div className="flex items-center gap-4">
                    <div className="glass px-6 py-4 rounded-2xl flex items-center gap-4 border border-white/5">
                        <div className="text-right">
                            <div className="text-[10px] font-bold text-muted-foreground uppercase tracking-widest">Global Graph</div>
                            <div className="text-xl font-bold tabular-nums">
                                {data ? `${data.nodes.length} Nodes` : '--'}
                            </div>
                        </div>
                        <div className="w-px h-8 bg-white/10" />
                        <div className="text-right">
                            <div className="text-[10px] font-bold text-muted-foreground uppercase tracking-widest">Active Links</div>
                            <div className="text-xl font-bold tabular-nums">
                                {data ? `${data.links.length} Edges` : '--'}
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            {/* Main Content */}
            <div className="relative h-[calc(100vh-320px)] min-h-[500px]">
                {loading ? (
                    <div className="w-full h-full glass rounded-3xl flex flex-col items-center justify-center gap-4">
                        <div className="w-12 h-12 border-4 border-accent/20 border-t-accent rounded-full animate-spin" />
                        <p className="text-sm font-bold uppercase tracking-widest text-muted-foreground animate-pulse">Calculating Graph Layout...</p>
                    </div>
                ) : error ? (
                    <div className="w-full h-full glass rounded-3xl flex flex-col items-center justify-center gap-4 text-red-400">
                        <Activity className="w-12 h-12" />
                        <p className="font-bold uppercase tracking-widest">Telemetry Data Unavailable</p>
                        <p className="text-sm text-muted-foreground">{error}</p>
                    </div>
                ) : (
                    <NetworkGraph data={data} />
                )}

                {/* Floating Info Card */}
                <div className="mt-8 grid grid-cols-1 md:grid-cols-3 gap-6">
                    <div className="p-6 glass border border-white/5 rounded-2xl">
                        <div className="flex items-center gap-2 mb-4 text-accent">
                            <Info className="w-4 h-4" />
                            <h3 className="font-bold text-[10px] uppercase tracking-widest">Clusters</h3>
                        </div>
                        <p className="text-xs text-muted-foreground leading-relaxed">
                            Nodes that gravitate together represent a shared ecosystem. Larger nodes indicate high-volume anchors or assets with multiple active trustlines.
                        </p>
                    </div>
                    <div className="p-6 glass border border-white/5 rounded-2xl">
                        <div className="flex items-center gap-2 mb-4 text-amber-400">
                            <Activity className="w-4 h-4" />
                            <h3 className="font-bold text-[10px] uppercase tracking-widest">Corridors</h3>
                        </div>
                        <p className="text-xs text-muted-foreground leading-relaxed">
                            Lines between assets represent corridors. Thickness is proportional to USD liquidity depth, while color reflects recent payment success rates.
                        </p>
                    </div>
                    <div className="p-6 glass border border-white/5 rounded-2xl">
                        <div className="flex items-center gap-2 mb-4 text-green-400">
                            <Share2 className="w-4 h-4" />
                            <h3 className="font-bold text-[10px] uppercase tracking-widest">Navigation</h3>
                        </div>
                        <p className="text-xs text-muted-foreground leading-relaxed">
                            Drag nodes to reorganize the layout. Use the mouse wheel to zoom. Hover over any element to view detailed performance metrics.
                        </p>
                    </div>
                </div>
            </div>
        </div>
    );
}
