"use client"

import React from 'react';
import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';

interface SettlementData {
    time: string;
    speed: number;
}

interface SettlementSpeedChartProps {
    data: SettlementData[];
}

export const SettlementSpeedChart: React.FC<SettlementSpeedChartProps> = ({ data }) => {
    return (
        <div className="bg-card text-card-foreground rounded-xl border shadow-sm">
            <div className="flex flex-col space-y-1.5 p-6">
                <h3 className="font-semibold leading-none tracking-tight">Average Settlement Speed</h3>
                <p className="text-sm text-muted-foreground">Seconds to finality over the last 24 hours.</p>
            </div>
            <div className="p-6 pt-0 h-[300px]">
                <ResponsiveContainer width="100%" height="100%">
                    <BarChart data={data} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
                        <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#E5E7EB" />
                        <XAxis
                            dataKey="time"
                            axisLine={false}
                            tickLine={false}
                            tickMargin={10}
                            tick={{ fontSize: 12, fill: "#6B7280" }}
                        />
                        <YAxis
                            axisLine={false}
                            tickLine={false}
                            tickFormatter={(value) => `${value}s`}
                            tick={{ fontSize: 12, fill: "#6B7280" }}
                        />
                        <Tooltip
                            cursor={{ fill: 'transparent' }}
                            contentStyle={{ borderRadius: '8px', border: '1px solid #E5E7EB', boxShadow: '0 1px 2px 0 rgba(0, 0, 0, 0.05)' }}
                            formatter={(value: number) => [`${value}s`, "Speed"]}
                        />
                        <Bar
                            dataKey="speed"
                            fill="#3b82f6"
                            radius={[4, 4, 0, 0]}
                            barSize={30}
                        />
                    </BarChart>
                </ResponsiveContainer>
            </div>
        </div>
    );
};
