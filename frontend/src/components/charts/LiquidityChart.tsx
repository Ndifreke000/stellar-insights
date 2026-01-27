'use client';

import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { LiquidityDataPoint } from '@/lib/analytics-api';

interface LiquidityChartProps {
  data: LiquidityDataPoint[];
}

export function LiquidityChart({ data }: LiquidityChartProps) {
  // Group data by corridor for better visualization
  const corridorData = new Map<
    string,
    { timestamp: string; liquidity_usd: number }[]
  >();

  data.forEach((point) => {
    if (!corridorData.has(point.corridor_key)) {
      corridorData.set(point.corridor_key, []);
    }
    corridorData.get(point.corridor_key)!.push({
      timestamp: point.timestamp,
      liquidity_usd: point.liquidity_usd,
    });
  });

  // Use aggregated data or show just top corridor for clarity
  const aggregatedData = new Map<string, number>();
  data.forEach((point) => {
    const timestamp = point.timestamp.split('T')[0]; // Date only
    const current = aggregatedData.get(timestamp) || 0;
    aggregatedData.set(timestamp, current + point.liquidity_usd);
  });

  const chartData = Array.from(aggregatedData.entries())
    .map(([timestamp, liquidity]) => ({
      timestamp: new Date(timestamp).toLocaleDateString('en-US', {
        month: 'short',
        day: 'numeric',
      }),
      liquidity_usd: Math.round(liquidity),
    }))
    .sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());

  const formatCurrency = (value: number) => {
    if (value >= 1000000) {
      return `$${(value / 1000000).toFixed(1)}M`;
    }
    return `$${(value / 1000).toFixed(0)}K`;
  };

  return (
    <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
      <h2 className="text-lg font-bold text-gray-900 dark:text-white mb-2">
        Liquidity Over Time
      </h2>
      <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
        Total available liquidity across all corridors
      </p>
      <ResponsiveContainer width="100%" height={350}>
        <LineChart data={chartData}>
          <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
          <XAxis
            dataKey="timestamp"
            stroke="#6b7280"
            tick={{ fontSize: 12 }}
          />
          <YAxis
            stroke="#6b7280"
            tickFormatter={formatCurrency}
            tick={{ fontSize: 12 }}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: '#1f2937',
              border: '1px solid #4b5563',
              borderRadius: '0.5rem',
            }}
            labelStyle={{ color: '#fff' }}
            formatter={(value: number) => formatCurrency(value)}
          />
          <Line
            type="monotone"
            dataKey="liquidity_usd"
            stroke="#3b82f6"
            strokeWidth={2}
            dot={false}
            name="Liquidity"
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
