"use client";

import React, { useState, useEffect } from "react";
import { Waves, Droplets, ArrowRight, Activity, Zap } from "lucide-react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { LiquidityChart } from "@/components/charts/LiquidityChart";
import { LiquidityHeatmap } from "@/components/charts/LiquidityHeatmap";
import { Badge } from "@/components/ui/badge";

export default function LiquidityPage() {
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        const timer = setTimeout(() => setLoading(false), 800);
        return () => clearTimeout(timer);
    }, []);

    if (loading) {
        return (
            <div className="flex h-[80vh] items-center justify-center">
                <div className="text-sm font-mono text-accent animate-pulse uppercase tracking-widest italic">Measuring Liquidity Depth... // 303-D</div>
            </div>
        );
    }

    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
            {/* Page Header */}
            <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
                <div>
                    <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">Market Dynamics // 05</div>
                    <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
                        <Waves className="w-8 h-8 text-accent" />
                        Liquidity Terminal
                    </h2>
                </div>
                <div className="flex items-center gap-3">
                    <Badge variant="outline" className="text-[10px] font-mono border-border/50 px-3 py-1 bg-accent/5">
                        HIGH_ACCURACY_STREAM
                    </Badge>
                </div>
            </div>

            {/* Overview Metrics */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <MetricCard
                    label="Total Liquidity Depth"
                    value="$124.5M"
                    subLabel="Aggregated Across Anchors"
                    trend={12.4}
                    trendDirection="up"
                />
                <MetricCard
                    label="Slippage Index"
                    value="0.04%"
                    subLabel="Avg. $10k Swap"
                    inverse
                    trend={-0.01}
                    trendDirection="down"
                />
                <MetricCard
                    label="Reserve Ratio"
                    value="1.42x"
                    subLabel="System Buffer"
                />
            </div>

            {/* Main Charts */}
            <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
                <div className="lg:col-span-8 glass-card rounded-2xl p-1">
                    <div className="p-6 pb-2">
                        <h3 className="text-xs font-mono text-muted-foreground uppercase tracking-widest mb-4 flex items-center gap-2">
                            <Activity className="w-3 h-3 text-accent" />
                            Global Liquidity Flux (24h)
                        </h3>
                    </div>
                    <LiquidityChart data={[]} /> {/* Empty array for fallback/mock */}
                </div>

                <div className="lg:col-span-4 glass-card rounded-2xl p-1">
                    <div className="p-6 pb-2">
                        <h3 className="text-xs font-mono text-muted-foreground uppercase tracking-widest mb-4 flex items-center gap-2">
                            <Zap className="w-3 h-3 text-accent" />
                            Market Concentration
                        </h3>
                    </div>
                    <LiquidityHeatmap corridors={[]} onTimePeriodChange={() => { }} />
                </div>
            </div>

            {/* Liquidity Provisions */}
            <div className="glass-card rounded-3xl p-8 border-dashed border-border/30">
                <div className="flex flex-col items-center justify-center py-12 text-center">
                    <Droplets className="w-12 h-12 text-accent/20 mb-4" />
                    <h3 className="text-lg font-black tracking-tight uppercase italic mb-2">Provisioning Engine</h3>
                    <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest max-w-xs">
                        Automated liquidity rebalancing is active. Directing capital to underserved corridors.
                    </p>
                    <button className="mt-6 px-8 py-3 bg-slate-900 border border-border/50 rounded-xl text-[10px] font-bold uppercase tracking-widest hover:border-accent/50 transition-all flex items-center gap-3">
                        Capital Allocation Map
                        <ArrowRight className="w-3 h-3 text-accent" />
                    </button>
                </div>
            </div>
        </div>
    );
}
