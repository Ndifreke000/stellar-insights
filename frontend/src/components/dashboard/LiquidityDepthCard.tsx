import React from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
  Legend,
} from 'recharts';

interface TimePoint {
  ts: string;
  tvl: number;
}

interface LiquidityDepthCardProps {
  data: TimePoint[];
}

export function LiquidityDepthCard({ data }: LiquidityDepthCardProps) {
  return (
    <div className="col-span-1 lg:col-span-2 bg-white rounded shadow p-4">
      <h2 className="text-sm text-muted-foreground">
        Liquidity Depth / TVL (24h)
      </h2>
      <div style={{ width: "100%", height: 240 }} className="mt-3">
        <ResponsiveContainer>
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis
              dataKey="ts"
              tickFormatter={(s) => new Date(s).getHours() + ":00"}
            />
            <YAxis />
            <Tooltip
              labelFormatter={(s) => new Date(s).toLocaleString()}
            />
            <Legend />
            <Line
              type="monotone"
              dataKey="tvl"
              stroke="#82ca9d"
              dot={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
