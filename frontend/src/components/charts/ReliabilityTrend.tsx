'use client';

import { useState, useMemo } from 'react';
import {
    AreaChart,
    Area,
    XAxis,
    YAxis,
    CartesianGrid,
    Tooltip,
    TooltipProps,
    ResponsiveContainer
} from 'recharts';
import { ReliabilityDataPoint } from '@/lib/api';
import { TooltipProps } from 'recharts';
import { NameType, ValueType } from 'recharts/types/component/DefaultTooltipContent';

interface ReliabilityTrendProps {
    data: ReliabilityDataPoint[];
}

type TimeWindow = '7d' | '30d' | '90d';

const CustomTooltip = ({ active, payload, label }: TooltipProps<number, string>) => {
    if (active && payload && payload.length) {
        return (
            <div className="bg-slate-900 border border-slate-700 p-3 rounded-lg shadow-xl">
                <p className="text-slate-400 text-xs mb-1">{label}</p>
                <p className="text-emerald-400 font-bold text-sm">
                    Score: {payload[0].value?.toFixed(1) ?? 'N/A'}
                </p>
            </div>
        );
    }
    return null;
};

export function ReliabilityTrend({ data }: ReliabilityTrendProps) {
    const [timeWindow, setTimeWindow] = useState<TimeWindow>('30d');

    const filteredData = useMemo(() => {
        if (!data || data.length === 0) return [];

        // Sort by date just in case
        const sortedData = [...data].sort((a, b) =>
            new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
        );

        const days = timeWindow === '7d' ? 7 : timeWindow === '90d' ? 90 : 30;
        return sortedData.slice(-days);
    }, [data, timeWindow]);

    return (
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-6 shadow-sm h-full">
            <div className="flex flex-row justify-between items-center mb-6">
                <div>
                    <h3 className="font-semibold text-white">Reliability Trend</h3>
                    <p className="text-sm text-slate-400 mt-1">Historical performance over time</p>
                </div>

                <div className="flex bg-slate-950 p-1 rounded-lg border border-slate-800">
                    {(['7d', '30d', '90d'] as TimeWindow[]).map((window) => (
                        <button
                            key={window}
                            onClick={() => setTimeWindow(window)}
                            className={`px-3 py-1 text-xs font-medium rounded-md transition-all ${timeWindow === window
                                ? 'bg-slate-800 text-white shadow-sm'
                                : 'text-slate-500 hover:text-slate-300'
                                }`}
                        >
                            {window}
                        </button>
                    ))}
                </div>
            </div>

            <div className="h-[250px] w-full">
                <ResponsiveContainer width="100%" height="100%">
                    <AreaChart
                        data={filteredData}
                        margin={{ top: 5, right: 0, left: 0, bottom: 0 }}
                    >
                        <defs>
                            <linearGradient id="colorScore" x1="0" y1="0" x2="0" y2="1">
                                <stop offset="5%" stopColor="#34d399" stopOpacity={0.3} />
                                <stop offset="95%" stopColor="#34d399" stopOpacity={0} />
                            </linearGradient>
                        </defs>
                        <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" vertical={false} />
                        <XAxis
                            dataKey="timestamp"
                            tick={{ fontSize: 12, fill: '#64748b' }}
                            tickLine={false}
                            axisLine={false}
                            tickMargin={10}
                            tickFormatter={(value) => {
                                const date = new Date(value);
                                return `${date.getMonth() + 1}/${date.getDate()}`;
                            }}
                        />
                        <YAxis
                            domain={[60, 100]}
                            tick={{ fontSize: 12, fill: '#64748b' }}
                            tickLine={false}
                            axisLine={false}
                            tickMargin={10}
                        />
                        <Tooltip content={CustomTooltip} />
                        <Area
                            type="monotone"
                            dataKey="score"
                            stroke="#34d399"
                            strokeWidth={2}
                            fillOpacity={1}
                            fill="url(#colorScore)"
                        />
                    </AreaChart>
                </ResponsiveContainer>
            </div>
        </div>
    );
}
