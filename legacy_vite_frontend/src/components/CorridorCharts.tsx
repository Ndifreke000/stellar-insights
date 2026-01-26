import React from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  BarChart,
  Bar,
} from 'recharts';
import { SuccessRateDataPoint, LatencyDataPoint, LiquidityDataPoint } from '@/lib/api';

interface SuccessRateChartProps {
  data: SuccessRateDataPoint[];
}

export function SuccessRateChart({ data }: SuccessRateChartProps) {
  return (
    <div className="w-full h-full">
      <h3 className="text-lg font-semibold mb-4 text-foreground">
        Historical Success Rate (30 days)
      </h3>
      <ResponsiveContainer width="100%" height={300}>
        <LineChart data={data}>
          <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
          <XAxis
            dataKey="timestamp"
            tick={{ fontSize: 12, fill: 'hsl(var(--muted-foreground))' }}
            interval={Math.floor(data.length / 6)}
            stroke="hsl(var(--border))"
          />
          <YAxis
            label={{ value: 'Success Rate (%)', angle: -90, position: 'insideLeft', fill: 'hsl(var(--muted-foreground))' }}
            domain={[0, 100]}
            tick={{ fill: 'hsl(var(--muted-foreground))' }}
            stroke="hsl(var(--border))"
          />
          <Tooltip
            formatter={(value: number) => [`${value.toFixed(2)}%`, 'Success Rate']}
            labelFormatter={(label: string) => `Date: ${label}`}
            contentStyle={{
              backgroundColor: 'hsl(var(--card))',
              border: '1px solid hsl(var(--border))',
              borderRadius: '8px',
              color: 'hsl(var(--foreground))',
            }}
          />
          <Legend />
          <Line
            type="monotone"
            dataKey="success_rate"
            stroke="hsl(var(--primary))"
            dot={false}
            strokeWidth={2}
            name="Success Rate %"
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

interface LatencyDistributionChartProps {
  data: LatencyDataPoint[];
}

export function LatencyDistributionChart({ data }: LatencyDistributionChartProps) {
  const chartData = data.map((item) => ({
    ...item,
    label: `${item.latency_bucket_ms}ms`,
  }));

  return (
    <div className="w-full h-full">
      <h3 className="text-lg font-semibold mb-4 text-foreground">
        Latency Distribution
      </h3>
      <ResponsiveContainer width="100%" height={300}>
        <BarChart data={chartData}>
          <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
          <XAxis
            dataKey="label"
            tick={{ fontSize: 12, fill: 'hsl(var(--muted-foreground))' }}
            stroke="hsl(var(--border))"
          />
          <YAxis
            label={{ value: 'Payment Count', angle: -90, position: 'insideLeft', fill: 'hsl(var(--muted-foreground))' }}
            tick={{ fill: 'hsl(var(--muted-foreground))' }}
            stroke="hsl(var(--border))"
          />
          <Tooltip
            formatter={(value: number) => [value.toLocaleString(), 'Count']}
            contentStyle={{
              backgroundColor: 'hsl(var(--card))',
              border: '1px solid hsl(var(--border))',
              borderRadius: '8px',
              color: 'hsl(var(--foreground))',
            }}
          />
          <Legend />
          <Bar dataKey="count" fill="#22c55e" name="Count" radius={[4, 4, 0, 0]} />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

interface LiquidityTrendChartProps {
  data: LiquidityDataPoint[];
}

export function LiquidityTrendChart({ data }: LiquidityTrendChartProps) {
  return (
    <div className="w-full h-full">
      <h3 className="text-lg font-semibold mb-4 text-foreground">
        Liquidity Trends (30 days)
      </h3>
      <ResponsiveContainer width="100%" height={300}>
        <LineChart data={data}>
          <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
          <XAxis
            dataKey="timestamp"
            tick={{ fontSize: 12, fill: 'hsl(var(--muted-foreground))' }}
            interval={Math.floor(data.length / 6)}
            stroke="hsl(var(--border))"
          />
          <YAxis
            yAxisId="left"
            tick={{ fill: 'hsl(var(--muted-foreground))' }}
            stroke="hsl(var(--border))"
          />
          <YAxis
            yAxisId="right"
            orientation="right"
            tick={{ fill: 'hsl(var(--muted-foreground))' }}
            stroke="hsl(var(--border))"
          />
          <Tooltip
            formatter={(value: number) => [`$${(value / 1000000).toFixed(2)}M`, '']}
            labelFormatter={(label: string) => `Date: ${label}`}
            contentStyle={{
              backgroundColor: 'hsl(var(--card))',
              border: '1px solid hsl(var(--border))',
              borderRadius: '8px',
              color: 'hsl(var(--foreground))',
            }}
          />
          <Legend />
          <Line
            yAxisId="left"
            type="monotone"
            dataKey="liquidity_usd"
            stroke="#a855f7"
            dot={false}
            strokeWidth={2}
            name="Liquidity Depth"
          />
          <Line
            yAxisId="right"
            type="monotone"
            dataKey="volume_24h_usd"
            stroke="#f59e0b"
            dot={false}
            strokeWidth={2}
            name="24h Volume"
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
