"use client"

import React from 'react';
import { Area, AreaChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';

interface LiquidityData {
    date: string;
    value: number;
}

interface LiquidityChartProps {
    data: LiquidityData[];
}

export const LiquidityChart: React.FC<LiquidityChartProps> = ({ data }) => {
    return (
        <div className="bg-card text-card-foreground rounded-xl border shadow-sm col-span-2">
            <div className="flex flex-col space-y-1.5 p-6">
                <h3 className="font-semibold leading-none tracking-tight">Network Liquidity Depth</h3>
                <p className="text-sm text-muted-foreground">Total Value Locked (TVL) trend over the last 6 months.</p>
            </div>
            <div className="p-6 pt-0 h-[300px]">
                <ResponsiveContainer width="100%" height="100%">
                    <AreaChart data={data} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
                        <defs>
                            <linearGradient id="colorValue" x1="0" y1="0" x2="0" y2="1">
                                <stop offset="5%" stopColor="#8884d8" stopOpacity={0.3} />
                                <stop offset="95%" stopColor="#8884d8" stopOpacity={0} />
                            </linearGradient>
                        </defs>
                        <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#E5E7EB" />
                        <XAxis
                            dataKey="date"
                            axisLine={false}
                            tickLine={false}
                            tickMargin={10}
                            tick={{ fontSize: 12, fill: "#6B7280" }}
                        />
                        <YAxis
                            axisLine={false}
                            tickLine={false}
                            tickFormatter={(value) => `$${(value / 1000000).toFixed(0)}M`}
                            tick={{ fontSize: 12, fill: "#6B7280" }}
                        />
                        <Tooltip
                            contentStyle={{ borderRadius: '8px', border: '1px solid #E5E7EB', boxShadow: '0 1px 2px 0 rgba(0, 0, 0, 0.05)' }}
                            formatter={(value: number) => [`$${(value / 1000000).toFixed(1)}M`, "Liquidity"]}
                        />
                        <Area
                            type="monotone"
                            dataKey="value"
                            stroke="#8884d8"
                            strokeWidth={2}
                            fillOpacity={1}
                            fill="url(#colorValue)"
                        />
                    </AreaChart>
                </ResponsiveContainer>
            </div>
        </div>
    );
};
