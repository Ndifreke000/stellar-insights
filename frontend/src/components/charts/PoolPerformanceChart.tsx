"use client";

import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from "recharts";
import { PoolSnapshot } from "@/lib/liquidity-pool-api";

interface PoolPerformanceChartProps {
  snapshots: PoolSnapshot[];
  metric?: "apy" | "volume" | "fees" | "tvl";
}

export function PoolPerformanceChart({
  snapshots,
  metric = "apy",
}: PoolPerformanceChartProps) {
  const sortedSnapshots = [...snapshots].sort(
    (a, b) =>
      new Date(a.snapshot_at).getTime() - new Date(b.snapshot_at).getTime(),
  );

  const chartData = sortedSnapshots.map((s) => ({
    timestamp: new Date(s.snapshot_at).toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
    }),
    apy: Math.round(s.apy * 100) / 100,
    volume: Math.round(s.volume_usd),
    fees: Math.round(s.fees_usd * 100) / 100,
    tvl: Math.round(s.total_value_usd),
    il: Math.round(s.impermanent_loss_pct * 100) / 100,
  }));

  const formatValue = (value: number) => {
    if (metric === "apy") return `${value.toFixed(2)}%`;
    return new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: "USD",
      notation: "compact",
      maximumFractionDigits: 1,
    }).format(value);
  };

  const lineConfigs: Record<
    string,
    { key: string; color: string; label: string }
  > = {
    apy: { key: "apy", color: "#10b981", label: "APY %" },
    volume: { key: "volume", color: "#6366f1", label: "Volume" },
    fees: { key: "fees", color: "#f59e0b", label: "Fees" },
    tvl: { key: "tvl", color: "#06b6d4", label: "TVL" },
  };

  const config = lineConfigs[metric];

  return (
    <div className="glass-card rounded-2xl p-6 border border-border/50">
      <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
        Pool Analytics // 06.A
      </div>
      <h2 className="text-xl font-black tracking-tighter uppercase italic mb-2">
        {config.label} Over Time
      </h2>
      <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest mb-6">
        Historical {config.label.toLowerCase()} performance tracking
      </p>

      <div className="h-[350px] w-full">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={chartData}>
            <CartesianGrid
              strokeDasharray="3 3"
              stroke="rgba(255,255,255,0.05)"
              vertical={false}
            />
            <XAxis
              dataKey="timestamp"
              stroke="rgba(255,255,255,0.3)"
              tick={{ fontSize: 10, fontFamily: "monospace" }}
              axisLine={false}
              tickLine={false}
              dy={10}
            />
            <YAxis
              stroke="rgba(255,255,255,0.3)"
              tickFormatter={formatValue}
              tick={{ fontSize: 10, fontFamily: "monospace" }}
              axisLine={false}
              tickLine={false}
              dx={-10}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "rgba(15, 23, 42, 0.95)",
                border: "1px solid rgba(255, 255, 255, 0.1)",
                borderRadius: "12px",
                backdropFilter: "blur(12px)",
                fontSize: "10px",
                fontFamily: "monospace",
                textTransform: "uppercase" as const,
              }}
              itemStyle={{ color: config.color, fontWeight: "bold" }}
              labelStyle={{ color: "#94a3b8", marginBottom: "4px" }}
              formatter={(value: number) => [
                formatValue(value),
                config.label.toUpperCase(),
              ]}
            />
            <Legend
              iconType="circle"
              wrapperStyle={{
                fontSize: "10px",
                fontFamily: "monospace",
                textTransform: "uppercase",
              }}
            />
            <Line
              type="monotone"
              dataKey={config.key}
              stroke={config.color}
              strokeWidth={3}
              dot={false}
              activeDot={{
                r: 4,
                fill: config.color,
                stroke: "#fff",
                strokeWidth: 2,
              }}
              name={config.label}
            />
            {metric === "apy" && (
              <Line
                type="monotone"
                dataKey="il"
                stroke="#ef4444"
                strokeWidth={2}
                strokeDasharray="5 5"
                dot={false}
                activeDot={{
                  r: 3,
                  fill: "#ef4444",
                  stroke: "#fff",
                  strokeWidth: 2,
                }}
                name="IL %"
              />
            )}
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
