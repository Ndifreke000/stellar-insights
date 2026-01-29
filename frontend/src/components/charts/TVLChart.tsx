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
import { TVLDataPoint } from '@/lib/analytics-api';

interface TVLChartProps {
  data: TVLDataPoint[];
}

export function TVLChart({ data }: TVLChartProps) {
  const chartData = data.map((point) => ({
    timestamp: new Date(point.timestamp).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
    }),
    tvl_usd: Math.round(point.tvl_usd),
  }));

  const formatCurrency = (value: number) => {
    if (value >= 1000000) {
      return `$${(value / 1000000).toFixed(1)}M`;
    }
    return `$${(value / 1000).toFixed(0)}K`;
  };

  const maxTVL = Math.max(...chartData.map((d) => d.tvl_usd));
  const minTVL = Math.min(...chartData.map((d) => d.tvl_usd));
  const avgTVL = chartData.reduce((sum, d) => sum + d.tvl_usd, 0) / chartData.length;

  return (
    <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
      <h2 className="text-lg font-bold text-gray-900 dark:text-white mb-2">
        Total Value Locked (TVL)
      </h2>
      <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
        Overall TVL across the Stellar network
      </p>

      {/* Summary Stats */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <div>
          <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
            Current
          </p>
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {formatCurrency(chartData[chartData.length - 1]?.tvl_usd || 0)}
          </p>
        </div>
        <div>
          <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
            Average
          </p>
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {formatCurrency(avgTVL)}
          </p>
        </div>
        <div>
          <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
            Range
          </p>
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {formatCurrency(maxTVL - minTVL)}
          </p>
        </div>
      </div>

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
            dataKey="tvl_usd"
            stroke="#10b981"
            strokeWidth={2}
            dot={false}
            name="TVL"
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
