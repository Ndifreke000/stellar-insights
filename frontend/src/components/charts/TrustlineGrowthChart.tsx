"use client";

import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Area,
  AreaChart,
} from "recharts";
import { TrustlineSnapshot } from "@/lib/trustline-api";

interface TrustlineGrowthChartProps {
  data: TrustlineSnapshot[];
  latestTotal: number;
}

export function TrustlineGrowthChart({
  data,
  latestTotal,
}: TrustlineGrowthChartProps) {
  if (!data || data.length === 0) {
    return (
      <div className="glass-card rounded-2xl p-6 border border-border/50 flex flex-col items-center justify-center h-[400px]">
        <h2 className="text-xl font-black tracking-tighter uppercase italic mb-2 opacity-50">
          Trustline Growth
        </h2>
        <p className="text-sm font-mono text-muted-foreground uppercase tracking-widest text-center">
          No historical data available for this asset
        </p>
      </div>
    );
  }

  // Sort and format data for Recharts
  const chartData = [...data]
    .sort(
      (a, b) =>
        new Date(a.snapshot_at).getTime() - new Date(b.snapshot_at).getTime(),
    )
    .map((point) => ({
      timestamp: new Date(point.snapshot_at).toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
      }),
      total: point.total_trustlines,
      authorized: point.authorized_trustlines,
    }));

  const formatNumber = (value: number) => {
    return new Intl.NumberFormat("en-US", {
      notation: "compact",
      maximumFractionDigits: 1,
    }).format(value);
  };

  const earliestTotal = chartData[0]?.total || 0;
  const growth = latestTotal - earliestTotal;
  const growthPercent = earliestTotal > 0 ? (growth / earliestTotal) * 100 : 0;

  return (
    <div className="glass-card rounded-2xl p-6 border border-border/50">
      <div className="flex flex-col md:flex-row md:items-start justify-between mb-8 gap-4">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2 cursor-default">
            Asset Adoption // 04.A
          </div>
          <h2 className="text-xl font-black tracking-tighter uppercase italic mb-2 cursor-default">
            Trustline Growth
          </h2>
          <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest cursor-default">
            Historical trustline counts for the asset
          </p>
        </div>

        <div className="flex gap-2 sm:gap-4 w-full md:w-auto mt-4 md:mt-0">
          <div className="p-3 rounded-xl bg-slate-900/30 border border-white/5 flex-1 sm:flex-none sm:min-w-[120px]">
            <p className="text-[9px] font-mono text-muted-foreground uppercase tracking-wider mb-1 cursor-default">
              Current
            </p>
            <p className="text-lg sm:text-xl font-black font-mono tracking-tighter text-foreground/90 cursor-default">
              {formatNumber(latestTotal)}
            </p>
          </div>
          <div className="p-3 rounded-xl bg-slate-900/30 border border-white/5 flex-1 sm:flex-none sm:min-w-[120px]">
            <p className="text-[9px] font-mono text-muted-foreground uppercase tracking-wider mb-1 cursor-default">
              Growth (Period)
            </p>
            <p
              className={`text-lg sm:text-xl font-black font-mono tracking-tighter cursor-default ${
                growth >= 0 ? "text-emerald-400" : "text-red-400"
              }`}
            >
              {growth >= 0 ? "+" : ""}
              {growthPercent.toFixed(1)}%
            </p>
          </div>
        </div>
      </div>

      <div className="h-[300px] w-full mt-4">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart
            data={chartData}
            margin={{ top: 10, right: 10, left: 0, bottom: 0 }}
          >
            <defs>
              <linearGradient id="colorTotal" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#6366f1" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#6366f1" stopOpacity={0} />
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
              minTickGap={30}
            />
            <YAxis
              stroke="rgba(255,255,255,0.3)"
              tickFormatter={formatNumber}
              tick={{ fontSize: 10, fontFamily: "monospace" }}
              axisLine={false}
              tickLine={false}
              dx={-10}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "rgba(15, 23, 42, 0.9)",
                border: "1px solid rgba(255, 255, 255, 0.1)",
                borderRadius: "12px",
                backdropFilter: "blur(12px)",
                fontSize: "10px",
                fontFamily: "monospace",
              }}
              labelStyle={{ color: "#94a3b8", marginBottom: "4px" }}
              itemStyle={{ fontFamily: "monospace", padding: "2px 0" }}
            />
            <Area
              type="monotone"
              dataKey="total"
              name="Total Trustlines"
              stroke="#6366f1"
              strokeWidth={3}
              fillOpacity={1}
              fill="url(#colorTotal)"
              activeDot={{
                r: 4,
                fill: "#6366f1",
                stroke: "#fff",
                strokeWidth: 2,
              }}
            />
            <Line
              type="monotone"
              dataKey="authorized"
              name="Authorized"
              stroke="#10b981"
              strokeWidth={2}
              strokeDasharray="4 4"
              dot={false}
              activeDot={{
                r: 4,
                fill: "#10b981",
                stroke: "#fff",
                strokeWidth: 2,
              }}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>

      <div className="flex items-center gap-6 mt-6 pt-4 border-t border-white/5">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded bg-[#6366f1]" />
          <span className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider cursor-default">
            Total Trustlines
          </span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded bg-[#10b981]" />
          <span className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider cursor-default">
            Authorized
          </span>
        </div>
      </div>
    </div>
  );
}
