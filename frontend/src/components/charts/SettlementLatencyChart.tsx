'use client';

import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts';
import { SettlementLatencyDataPoint } from '@/lib/analytics-api';

interface SettlementLatencyChartProps {
  data: SettlementLatencyDataPoint[];
}

export function SettlementLatencyChart({ data }: SettlementLatencyChartProps) {
  const chartData = data.map((point) => ({
    timestamp: new Date(point.timestamp).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
    }),
    median: Math.round(point.median_latency_ms),
    p95: Math.round(point.p95_latency_ms),
    p99: Math.round(point.p99_latency_ms),
  }));

  const formatLatency = (value: number) => {
    return `${value}ms`;
  };

  const latestPoint = data[data.length - 1];

  return (
    <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
      <h2 className="text-lg font-bold text-gray-900 dark:text-white mb-2">
        Settlement Latency
      </h2>
      <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
        Median and percentile settlement times
      </p>

      {/* Summary Stats */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <div>
          <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
            Median (Latest)
          </p>
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {formatLatency(Math.round(latestPoint?.median_latency_ms || 0))}
          </p>
        </div>
        <div>
          <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
            P95 (Latest)
          </p>
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {formatLatency(Math.round(latestPoint?.p95_latency_ms || 0))}
          </p>
        </div>
        <div>
          <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
            P99 (Latest)
          </p>
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {formatLatency(Math.round(latestPoint?.p99_latency_ms || 0))}
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
            tickFormatter={formatLatency}
            tick={{ fontSize: 12 }}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: '#1f2937',
              border: '1px solid #4b5563',
              borderRadius: '0.5rem',
            }}
            labelStyle={{ color: '#fff' }}
            formatter={(value: number) => formatLatency(value)}
          />
          <Legend />
          <Line
            type="monotone"
            dataKey="median"
            stroke="#3b82f6"
            strokeWidth={2}
            dot={false}
            name="Median"
          />
          <Line
            type="monotone"
            dataKey="p95"
            stroke="#f59e0b"
            strokeWidth={2}
            dot={false}
            name="P95"
            strokeDasharray="5 5"
          />
          <Line
            type="monotone"
            dataKey="p99"
            stroke="#ef4444"
            strokeWidth={2}
            dot={false}
            name="P99"
            strokeDasharray="5 5"
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
