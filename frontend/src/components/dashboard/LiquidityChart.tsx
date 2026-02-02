"use client"

import React from 'react';
import { Area, AreaChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import { Badge } from '../ui/badge';

interface LiquidityData {
    date: string;
    value: number;
}

interface LiquidityChartProps {
    data: LiquidityData[];
}

export const LiquidityChart: React.FC<LiquidityChartProps> = ({ data }) => {
    return (
        <div className="p-6">
            <div className="flex items-center justify-between mb-6">
                <div>
                    <h3 className="text-sm font-bold uppercase tracking-widest text-muted-foreground">Liquidity Depth // TVL</h3>
                    <p className="text-[10px] text-muted-foreground/50 font-mono uppercase mt-1">Stellar Asset Ecosystem</p>
                </div>
                <Badge variant="outline" className="text-[10px] font-mono border-border/50">M/M ANALYSIS</Badge>
            </div>

            <div className="h-[300px] w-full mt-4">
                <ResponsiveContainer width="100%" height="100%">
                    <AreaChart data={data} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
                        <defs>
                            <linearGradient id="liquidityFill" x1="0" y1="0" x2="0" y2="1">
                                <stop offset="5%" stopColor="#6366f1" stopOpacity={0.2} />
                                <stop offset="95%" stopColor="#6366f1" stopOpacity={0} />
                            </linearGradient>
                        </defs>
                        <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#ffffff10" />
                        <XAxis
                            dataKey="date"
                            axisLine={false}
                            tickLine={false}
                            tickMargin={10}
                            tick={{ fontSize: 10, fill: "#94a3b8", fontWeight: 500, fontFamily: 'monospace' }}
                        />
                        <YAxis
                            axisLine={false}
                            tickLine={false}
                            tickFormatter={(value) => `$${(value / 1000000).toFixed(0)}M`}
                            tick={{ fontSize: 10, fill: "#94a3b8", fontWeight: 500, fontFamily: 'monospace' }}
                        />
                        <Tooltip
                            contentStyle={{
                                backgroundColor: 'rgba(15, 23, 42, 0.9)',
                                borderRadius: '12px',
                                border: '1px solid rgba(255, 255, 255, 0.1)',
                                backdropFilter: 'blur(8px)',
                                color: '#f8fafc',
                                fontSize: '12px',
                                fontFamily: 'monospace'
                            }}
                            cursor={{ stroke: 'rgba(99, 102, 241, 0.2)', strokeWidth: 1 }}
                            formatter={(value: number) => [`$${(value / 1000000).toFixed(2)}M`, "Total Liquidity"]}
                        />
                        <Area
                            type="monotone"
                            dataKey="value"
                            stroke="#6366f1"
                            strokeWidth={3}
                            fillOpacity={1}
                            fill="url(#liquidityFill)"
                            animationDuration={2000}
                        />
                    </AreaChart>
                </ResponsiveContainer>
            </div>
        </div>
    );
};
