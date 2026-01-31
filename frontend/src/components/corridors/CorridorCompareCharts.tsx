"use client";

import React from "react";
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
} from "recharts";
import { CorridorDetailData } from "@/lib/api";

interface CompareChartsProps {
  corridors: CorridorDetailData[];
}

const COLORS = ["#3b82f6", "#10b981", "#f59e0b"];

export function SuccessRateCompareChart({ corridors }: CompareChartsProps) {
  const allTimestamps = Array.from(
    new Set(
      corridors.flatMap((c) =>
        c.historical_success_rate.map((d) => d.timestamp),
      ),
    ),
  ).sort();

  const chartData = allTimestamps.map((ts) => {
    const dataPoint: Record<string, string | number> = { timestamp: ts };
    corridors.forEach((c) => {
      const point = c.historical_success_rate.find((d) => d.timestamp === ts);
      if (point) {
        dataPoint[c.corridor.id] = point.success_rate;
      }
    });
    return dataPoint;
  });

  return (
    <div className="w-full bg-white dark:bg-slate-800 rounded-lg shadow-md p-6 border border-gray-100 dark:border-slate-700">
      <h3 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">
        Success Rate Comparison (%)
      </h3>
      <ResponsiveContainer width="100%" height={300}>
        <LineChart data={chartData}>
          <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#334155" opacity={0.1} />
          <XAxis 
            dataKey="timestamp" 
            tick={{ fontSize: 11 }} 
            axisLine={false}
            tickLine={false}
          />
          <YAxis 
            domain={[0, 100]} 
            tick={{ fontSize: 11 }}
            axisLine={false}
            tickLine={false}
          />
          <Tooltip 
            contentStyle={{ 
              backgroundColor: '#1e293b', 
              border: 'none', 
              borderRadius: '8px',
              color: '#f8fafc'
            }}
            itemStyle={{ fontSize: '12px' }}
          />
          <Legend iconType="circle" wrapperStyle={{ paddingTop: '20px' }} />
          {corridors.map((c, i) => (
            <Line
              key={c.corridor.id}
              type="monotone"
              dataKey={c.corridor.id}
              stroke={COLORS[i % COLORS.length]}
              dot={false}
              strokeWidth={3}
              name={c.corridor.id}
              animationDuration={1500}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

export function VolumeCompareChart({ corridors }: CompareChartsProps) {
  const allTimestamps = Array.from(
    new Set(corridors.flatMap((c) => c.historical_volume.map((d) => d.timestamp))),
  ).sort();

  const chartData = allTimestamps.map((ts) => {
    const dataPoint: Record<string, string | number> = { timestamp: ts };
    corridors.forEach((c) => {
      const point = c.historical_volume.find((d) => d.timestamp === ts);
      if (point) {
        dataPoint[c.corridor.id] = point.volume_usd;
      }
    });
    return dataPoint;
  });

  return (
    <div className="w-full bg-white dark:bg-slate-800 rounded-lg shadow-md p-6 border border-gray-100 dark:border-slate-700">
      <h3 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">
        Volume Comparison (USD)
      </h3>
      <ResponsiveContainer width="100%" height={300}>
        <BarChart data={chartData}>
          <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#334155" opacity={0.1} />
          <XAxis 
            dataKey="timestamp" 
            tick={{ fontSize: 11 }}
            axisLine={false}
            tickLine={false}
          />
          <YAxis 
            tickFormatter={(value) => `$${(value / 1000).toFixed(0)}k`}
            tick={{ fontSize: 11 }}
            axisLine={false}
            tickLine={false}
          />
          <Tooltip 
            formatter={(value: number) => [`$${value.toLocaleString()}`, "Volume"]}
            contentStyle={{ 
              backgroundColor: '#1e293b', 
              border: 'none', 
              borderRadius: '8px',
              color: '#f8fafc'
            }}
          />
          <Legend iconType="circle" wrapperStyle={{ paddingTop: '20px' }} />
          {corridors.map((c, i) => (
            <Bar
              key={c.corridor.id}
              dataKey={c.corridor.id}
              fill={COLORS[i % COLORS.length]}
              name={c.corridor.id}
              radius={[4, 4, 0, 0]}
            />
          ))}
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

export function SlippageCompareChart({ corridors }: CompareChartsProps) {
  const allTimestamps = Array.from(
    new Set(
      corridors.flatMap((c) =>
        c.historical_slippage.map((d) => d.timestamp),
      ),
    ),
  ).sort();

  const chartData = allTimestamps.map((ts) => {
    const dataPoint: Record<string, string | number> = { timestamp: ts };
    corridors.forEach((c) => {
      const point = c.historical_slippage.find((d) => d.timestamp === ts);
      if (point) {
        dataPoint[c.corridor.id] = point.average_slippage_bps;
      }
    });
    return dataPoint;
  });

  return (
    <div className="w-full bg-white dark:bg-slate-800 rounded-lg shadow-md p-6 border border-gray-100 dark:border-slate-700">
      <h3 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">
        Slippage Comparison (bps)
      </h3>
      <ResponsiveContainer width="100%" height={300}>
        <LineChart data={chartData}>
          <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#334155" opacity={0.1} />
          <XAxis 
            dataKey="timestamp" 
            tick={{ fontSize: 11 }}
            axisLine={false}
            tickLine={false}
          />
          <YAxis 
            tick={{ fontSize: 11 }}
            axisLine={false}
            tickLine={false}
          />
          <Tooltip 
            formatter={(value: number) => [`${value.toFixed(2)} bps`, "Slippage"]}
            contentStyle={{ 
              backgroundColor: '#1e293b', 
              border: 'none', 
              borderRadius: '8px',
              color: '#f8fafc'
            }}
          />
          <Legend iconType="circle" wrapperStyle={{ paddingTop: '20px' }} />
          {corridors.map((c, i) => (
            <Line
              key={c.corridor.id}
              type="stepAfter"
              dataKey={c.corridor.id}
              stroke={COLORS[i % COLORS.length]}
              dot={false}
              strokeWidth={3}
              name={c.corridor.id}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
