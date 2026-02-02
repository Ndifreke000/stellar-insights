"use client"

import React from 'react';
import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis, Cell } from 'recharts';
import { Badge } from '../ui/badge';

interface SettlementData {
    time: string;
    speed: number;
}

interface SettlementSpeedChartProps {
    data: SettlementData[];
}

export const SettlementSpeedChart: React.FC<SettlementSpeedChartProps> = ({ data }) => {
    return (
        <div className="p-6">
            <div className="flex items-center justify-between mb-6">
                <div>
                    <h3 className="text-sm font-bold uppercase tracking-widest text-muted-foreground">Speed Distributions // Finality</h3>
                    <p className="text-[10px] text-muted-foreground/50 font-mono uppercase mt-1">Network Latency (24h)</p>
                </div>
                <Badge variant="outline" className="text-[10px] font-mono border-border/50">REAL_TIME</Badge>
            </div>

            <div className="h-[300px] w-full mt-4">
                <ResponsiveContainer width="100%" height="100%">
                    <BarChart data={data} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
                        <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#ffffff10" />
                        <XAxis
                            dataKey="time"
                            axisLine={false}
                            tickLine={false}
                            tickMargin={10}
                            tick={{ fontSize: 10, fill: "#94a3b8", fontWeight: 500, fontFamily: 'monospace' }}
                        />
                        <YAxis
                            axisLine={false}
                            tickLine={false}
                            tickFormatter={(value) => `${value}s`}
                            tick={{ fontSize: 10, fill: "#94a3b8", fontWeight: 500, fontFamily: 'monospace' }}
                        />
                        <Tooltip
                            cursor={{ fill: 'rgba(255, 255, 255, 0.05)' }}
                            contentStyle={{
                                backgroundColor: 'rgba(15, 23, 42, 0.9)',
                                borderRadius: '12px',
                                border: '1px solid rgba(255, 255, 255, 0.1)',
                                backdropFilter: 'blur(8px)',
                                color: '#f8fafc',
                                fontSize: '12px',
                                fontFamily: 'monospace'
                            }}
                            formatter={(value: number) => [`${value}s`, "Settlement Time"]}
                        />
                        <Bar
                            dataKey="speed"
                            radius={[4, 4, 0, 0]}
                            barSize={20}
                            animationDuration={1500}
                        >
                            {data.map((entry, index) => (
                                <Cell
                                    key={`cell-${index}`}
                                    fill={entry.speed > 5 ? '#f43f5e' : entry.speed > 3 ? '#6366f1' : '#10b981'}
                                    fillOpacity={0.8}
                                />
                            ))}
                        </Bar>
                    </BarChart>
                </ResponsiveContainer>
            </div>
        </div>
    );
};
