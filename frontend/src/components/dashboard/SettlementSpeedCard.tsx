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
  settlementMs: number;
}

interface SettlementSpeedCardProps {
  data: TimePoint[];
}

export function SettlementSpeedCard({ data }: SettlementSpeedCardProps) {
  return (
    <div className="col-span-1 lg:col-span-2 bg-white rounded shadow p-4">
      <h2 className="text-sm text-gray-500">
        Settlement Speed (ms) — last 24 points
      </h2>
      <div style={{ width: "100%", height: 220 }} className="mt-3">
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
              dataKey="settlementMs"
              stroke="#8884d8"
              dot={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
