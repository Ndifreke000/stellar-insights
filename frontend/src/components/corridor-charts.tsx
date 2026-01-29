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
import {
  SuccessRateDataPoint,
  LatencyDataPoint,
  LiquidityDataPoint,
  VolumeDataPoint,
  SlippageDataPoint,
} from "@/lib/api";

interface SuccessRateChartProps {
  data: SuccessRateDataPoint[];
}

export function SuccessRateChart({ data }: SuccessRateChartProps) {
  return (
    <div className="w-full h-full bg-white rounded-lg shadow-md p-6">
      <h3 className="text-lg font-semibold mb-4 text-gray-900">
        Historical Success Rate (30 days)
      </h3>
      <ResponsiveContainer width="100%" height={300}>
        <LineChart data={data}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis
            dataKey="timestamp"
            tick={{ fontSize: 12 }}
            interval={Math.floor(data.length / 6)}
          />
          <YAxis
            label={{
              value: "Success Rate (%)",
              angle: -90,
              position: "insideLeft",
            }}
            domain={[0, 100]}
          />
          <Tooltip
            formatter={(value, _name, _props) => {
              if (typeof value === "number") {
                return `${value.toFixed(2)}%`;
              }
              return value;
            }}
            labelFormatter={(label, _payload) => `Date: ${label}`}
          />
          <Legend />
          <Line
            type="monotone"
            dataKey="success_rate"
            stroke="#3b82f6"
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

export function LatencyDistributionChart({
  data,
}: LatencyDistributionChartProps) {
  const chartData = data.map((item) => ({
    ...item,
    label: `${item.latency_bucket_ms}ms`,
  }));

  return (
    <div className="w-full h-full bg-white rounded-lg shadow-md p-6">
      <h3 className="text-lg font-semibold mb-4 text-gray-900">
        Latency Distribution
      </h3>
      <ResponsiveContainer width="100%" height={300}>
        <BarChart data={chartData}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="label" tick={{ fontSize: 12 }} />
          <YAxis
            label={{
              value: "Payment Count",
              angle: -90,
              position: "insideLeft",
            }}
          />
          <Tooltip
            formatter={(value, _name, _props) => {
              if (typeof value === "number") {
                return value.toLocaleString();
              }
              return value;
            }}
          />
          <Legend />
          <Bar dataKey="count" fill="#10b981" name="Count" />
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
    <div className="w-full h-full bg-white rounded-lg shadow-md p-6">
      <h3 className="text-lg font-semibold mb-4 text-gray-900">
        Liquidity Trends (30 days)
      </h3>
      <ResponsiveContainer width="100%" height={300}>
        <LineChart data={data}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis
            dataKey="timestamp"
            tick={{ fontSize: 12 }}
            interval={Math.floor(data.length / 6)}
          />
          <YAxis yAxisId="left" />
          <YAxis yAxisId="right" orientation="right" />
          <Tooltip
            formatter={(value, _name, _props) => {
              if (typeof value === "number") {
                return `$${(value / 1000000).toFixed(2)}M`;
              }
              return value;
            }}
            labelFormatter={(label, _payload) => `Date: ${label}`}
          />
          <Legend />
          <Line
            yAxisId="left"
            type="monotone"
            dataKey="liquidity_usd"
            stroke="#8b5cf6"
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

interface VolumeTrendChartProps {
  data: VolumeDataPoint[];
}

export function VolumeTrendChart({ data }: VolumeTrendChartProps) {
  return (
    <div className="w-full h-full bg-white dark:bg-slate-800 rounded-lg shadow-md p-6">
      <h3 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">
        24h Volume Trends (30 days)
      </h3>
      <ResponsiveContainer width="100%" height={300}>
        <BarChart data={data}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis
            dataKey="timestamp"
            tick={{ fontSize: 12 }}
            interval={Math.floor(data.length / 6)}
          />
          <YAxis
            label={{ value: "Volume ($)", angle: -90, position: "insideLeft" }}
            tickFormatter={(value: number) => `$${(value / 1000).toFixed(0)}k`}
          />
          <Tooltip
            formatter={(value, _name, _props) => {
              if (typeof value === "number") {
                return `$${value.toLocaleString()}`;
              }
              return value;
            }}
            labelFormatter={(label, _payload) => `Date: ${label}`}
          />
          <Legend />
          <Bar dataKey="volume_usd" fill="#f59e0b" name="24h Volume" />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

interface SlippageTrendChartProps {
  data: SlippageDataPoint[];
}

export function SlippageTrendChart({ data }: SlippageTrendChartProps) {
  return (
    <div className="w-full h-full bg-white dark:bg-slate-800 rounded-lg shadow-md p-6">
      <h3 className="text-lg font-semibold mb-4 text-gray-900 dark:text-white">
        Average Slippage (30 days)
      </h3>
      <ResponsiveContainer width="100%" height={300}>
        <LineChart data={data}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis
            dataKey="timestamp"
            tick={{ fontSize: 12 }}
            interval={Math.floor(data.length / 6)}
          />
          <YAxis
            label={{
              value: "Slippage (bps)",
              angle: -90,
              position: "insideLeft",
            }}
            domain={["auto", "auto"]}
          />
          <Tooltip
            formatter={(value, _name, _props) => {
              if (typeof value === "number") {
                return `${value.toFixed(2)} bps`;
              }
              return value;
            }}
            labelFormatter={(label, _payload) => `Date: ${label}`}
          />
          <Legend />
          <Line
            type="monotone"
            dataKey="average_slippage_bps"
            stroke="#ef4444"
            dot={false}
            strokeWidth={2}
            name="Slippage (bps)"
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
