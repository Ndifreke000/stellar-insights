"use client";

import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from "recharts";
import { TrustlineSnapshot } from "@/lib/trustline-api";

interface TrustlineGrowthChartProps {
  snapshots: TrustlineSnapshot[];
  assetCode?: string;
}

export function TrustlineGrowthChart({
  snapshots,
  assetCode = "Asset",
}: TrustlineGrowthChartProps) {
  const sortedSnapshots = [...snapshots].sort(
    (a, b) =>
      new Date(a.snapshot_at).getTime() - new Date(b.snapshot_at).getTime(),
  );

  const chartData = sortedSnapshots.map((s) => ({
    timestamp: new Date(s.snapshot_at).toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
    }),
    trustlines: s.num_accounts,
    supply: Math.round(s.amount),
  }));

  const formatNumber = (value: number) =>
    new Intl.NumberFormat("en-US", {
      notation: "compact",
      maximumFractionDigits: 1,
    }).format(value);

  return (
    <div className="glass-card rounded-xl md:rounded-2xl p-4 md:p-6 border border-border/50">
      <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
        Trustline Analytics // 07.A
      </div>
      <h2 className="text-lg md:text-xl font-black tracking-tighter uppercase italic mb-2">
        {assetCode} Trustline Growth
      </h2>
      <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest mb-4 md:mb-6">
        Holder count and supply over time
      </p>

      <div className="h-[250px] md:h-[350px] w-full">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={chartData}>
            <defs>
              <linearGradient id="trustlineGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#6366f1" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#6366f1" stopOpacity={0} />
              </linearGradient>
              <linearGradient id="supplyGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#10b981" stopOpacity={0.2} />
                <stop offset="95%" stopColor="#10b981" stopOpacity={0} />
              </linearGradient>
            </defs>
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
              yAxisId="left"
              stroke="rgba(255,255,255,0.3)"
              tickFormatter={formatNumber}
              tick={{ fontSize: 10, fontFamily: "monospace" }}
              axisLine={false}
              tickLine={false}
              dx={-10}
            />
            <YAxis
              yAxisId="right"
              orientation="right"
              stroke="rgba(255,255,255,0.15)"
              tickFormatter={formatNumber}
              tick={{ fontSize: 10, fontFamily: "monospace" }}
              axisLine={false}
              tickLine={false}
              dx={10}
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
              labelStyle={{ color: "#94a3b8", marginBottom: "4px" }}
              formatter={(value: number, name: string) => [
                formatNumber(value),
                name === "trustlines" ? "TRUSTLINES" : "SUPPLY",
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
            <Area
              yAxisId="left"
              type="monotone"
              dataKey="trustlines"
              stroke="#6366f1"
              strokeWidth={3}
              fill="url(#trustlineGrad)"
              dot={false}
              activeDot={{
                r: 4,
                fill: "#6366f1",
                stroke: "#fff",
                strokeWidth: 2,
              }}
              name="Trustlines"
            />
            <Area
              yAxisId="right"
              type="monotone"
              dataKey="supply"
              stroke="#10b981"
              strokeWidth={2}
              strokeDasharray="5 5"
              fill="url(#supplyGrad)"
              dot={false}
              activeDot={{
                r: 3,
                fill: "#10b981",
                stroke: "#fff",
                strokeWidth: 2,
              }}
              name="Supply"
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
