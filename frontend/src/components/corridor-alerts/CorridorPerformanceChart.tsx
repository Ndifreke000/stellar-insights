"use client";

import { useMemo } from "react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  TooltipProps,
  Line,
  ComposedChart,
  Bar,
  ReferenceLine,
} from "recharts";
import type { CorridorPerformanceSnapshot } from "@/lib/alerts-api";

interface CorridorPerformanceChartProps {
  snapshots: CorridorPerformanceSnapshot[];
  metric: "success_rate" | "latency" | "liquidity" | "volume";
}

const CustomTooltip = (props: TooltipProps<number, string>) => {
  const { active, payload, label } = props;
  if (active && payload && payload.length) {
    return (
      <div className="bg-slate-900 border border-slate-700 p-3 rounded-lg shadow-xl">
        <p className="text-slate-400 text-xs mb-1">{label}</p>
        {payload.map((entry, idx) => (
          <p key={idx} className="font-bold text-sm" style={{ color: entry.color }}>
            {entry.name}: {typeof entry.value === "number" ? entry.value.toFixed(2) : entry.value}
          </p>
        ))}
      </div>
    );
  }
  return null;
};

export function CorridorPerformanceChart({ snapshots, metric }: CorridorPerformanceChartProps) {
  const data = useMemo(() => {
    return [...snapshots]
      .sort((a, b) => new Date(a.snapshot_time).getTime() - new Date(b.snapshot_time).getTime())
      .map((s) => ({
        time: new Date(s.snapshot_time).toLocaleString("en-US", {
          month: "short",
          day: "numeric",
          hour: "2-digit",
          minute: "2-digit",
        }),
        success_rate: s.success_rate * 100,
        latency: s.avg_settlement_latency_ms,
        liquidity: s.liquidity_depth_usd / 1000,
        volume: s.volume_usd / 1000,
      }));
  }, [snapshots]);

  if (data.length === 0) {
    return (
      <div className="h-64 flex items-center justify-center text-muted-foreground text-sm">
        No performance data available
      </div>
    );
  }

  switch (metric) {
    case "success_rate":
      return (
        <ResponsiveContainer width="100%" height={300}>
          <AreaChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.05)" />
            <XAxis dataKey="time" tick={{ fontSize: 10, fill: "#94a3b8" }} />
            <YAxis domain={[0, 100]} tick={{ fontSize: 10, fill: "#94a3b8" }} />
            <Tooltip content={<CustomTooltip />} />
            <ReferenceLine y={90} stroke="#22c55e" strokeDasharray="3 3" strokeOpacity={0.5} />
            <ReferenceLine y={80} stroke="#ef4444" strokeDasharray="3 3" strokeOpacity={0.5} />
            <Area
              type="monotone"
              dataKey="success_rate"
              stroke="#22c55e"
              fill="rgba(34,197,94,0.1)"
              strokeWidth={2}
              name="Success Rate (%)"
            />
          </AreaChart>
        </ResponsiveContainer>
      );

    case "latency":
      return (
        <ResponsiveContainer width="100%" height={300}>
          <ComposedChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.05)" />
            <XAxis dataKey="time" tick={{ fontSize: 10, fill: "#94a3b8" }} />
            <YAxis tick={{ fontSize: 10, fill: "#94a3b8" }} />
            <Tooltip content={<CustomTooltip />} />
            <Bar dataKey="latency" fill="rgba(234,179,8,0.3)" name="Latency (ms)" radius={[2, 2, 0, 0]} />
            <Line type="monotone" dataKey="latency" stroke="#eab308" strokeWidth={2} dot={false} name="Latency (ms)" />
          </ComposedChart>
        </ResponsiveContainer>
      );

    case "liquidity":
      return (
        <ResponsiveContainer width="100%" height={300}>
          <AreaChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.05)" />
            <XAxis dataKey="time" tick={{ fontSize: 10, fill: "#94a3b8" }} />
            <YAxis tick={{ fontSize: 10, fill: "#94a3b8" }} tickFormatter={(v) => `$${v}k`} />
            <Tooltip content={<CustomTooltip />} />
            <Area
              type="monotone"
              dataKey="liquidity"
              stroke="#3b82f6"
              fill="rgba(59,130,246,0.1)"
              strokeWidth={2}
              name="Liquidity ($k)"
            />
          </AreaChart>
        </ResponsiveContainer>
      );

    case "volume":
      return (
        <ResponsiveContainer width="100%" height={300}>
          <AreaChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.05)" />
            <XAxis dataKey="time" tick={{ fontSize: 10, fill: "#94a3b8" }} />
            <YAxis tick={{ fontSize: 10, fill: "#94a3b8" }} tickFormatter={(v) => `$${v}k`} />
            <Tooltip content={<CustomTooltip />} />
            <Area
              type="monotone"
              dataKey="volume"
              stroke="#8b5cf6"
              fill="rgba(139,92,246,0.1)"
              strokeWidth={2}
              name="Volume ($k)"
            />
          </AreaChart>
        </ResponsiveContainer>
      );
  }
}
