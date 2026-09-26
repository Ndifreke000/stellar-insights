"use client";

import { useEffect, useState } from "react";
import { RefreshCw, Clock, TrendingUp, AlertOctagon } from "lucide-react";
import dynamic from "next/dynamic";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { SkeletonCard, SkeletonChart } from "@/components/ui/Skeleton";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import {
  fetchSettlementDistribution,
  type SettlementDistributionData,
  type CorridorSettlementPercentiles,
  type SettlementTrendPoint,
} from "@/lib/analytics-api";
import {
  ResponsiveContainer,
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  Legend,
  CartesianGrid,
  TooltipValueType,
} from "recharts";

// ── Helpers ──────────────────────────────────────────────────────────────────

function formatMs(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
  return `${ms.toFixed(0)}ms`;
}

// ── Percentile bar row ───────────────────────────────────────────────────────

function PercentileRow({ corridor }: { corridor: CorridorSettlementPercentiles }) {
  const maxMs = corridor.max_ms || 1;
  return (
    <div className="glass-card rounded-xl p-4 space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-xs font-mono font-bold uppercase">{corridor.corridor_key}</span>
        <div className="flex items-center gap-2">
          {corridor.outlier_count > 0 && (
            <span className="text-[10px] text-red-400 font-mono flex items-center gap-1">
              <AlertOctagon className="w-3 h-3" />
              {corridor.outlier_count} outliers
            </span>
          )}
          <span className="text-[10px] text-muted-foreground font-mono">
            {corridor.sample_count.toLocaleString()} samples
          </span>
        </div>
      </div>

      {/* Stacked percentile bars */}
      <div className="space-y-1.5">
        {(
          [
            { label: "p50", value: corridor.p50_ms, color: "#22c55e" },
            { label: "p95", value: corridor.p95_ms, color: "#f97316" },
            { label: "p99", value: corridor.p99_ms, color: "#ef4444" },
          ] as const
        ).map(({ label, value, color }) => (
          <div key={label} className="flex items-center gap-2">
            <span className="text-[10px] font-mono w-6 text-muted-foreground">{label}</span>
            <div className="flex-1 h-2 rounded-full bg-white/10 overflow-hidden">
              <div
                className="h-full rounded-full transition-all duration-700"
                style={{
                  width: `${Math.min((value / maxMs) * 100, 100)}%`,
                  background: color,
                }}
              />
            </div>
            <span
              className="text-[10px] font-mono w-14 text-right font-bold"
              style={{ color }}
            >
              {formatMs(value)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ── Trend chart ───────────────────────────────────────────────────────────────

function TrendChart({ data }: { data: SettlementTrendPoint[] }) {
  // Sample down to at most 48 points for readability
  const sampled =
    data.length > 48
      ? data.filter((_, i) => i % Math.ceil(data.length / 48) === 0)
      : data;

  const chartData = sampled.map((d) => ({
    bucket: d.bucket.slice(5, 13).replace("T", " "), // "MM-DD HH"
    p50: Math.round(d.p50_ms),
    p95: Math.round(d.p95_ms),
    p99: Math.round(d.p99_ms),
  }));

  return (
    <div className="glass-card rounded-2xl p-6">
      <div className="flex items-center gap-2 mb-4">
        <TrendingUp className="w-4 h-4 text-accent" />
        <h3 className="text-xs font-mono font-bold uppercase tracking-widest">
          7-Day Settlement Trend (Network-wide)
        </h3>
      </div>
      <ResponsiveContainer width="100%" height={280}>
        <LineChart data={chartData} margin={{ top: 4, right: 12, bottom: 4, left: 0 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.05)" />
          <XAxis
            dataKey="bucket"
            tick={{ fontSize: 9, fontFamily: "monospace", fill: "#64748b" }}
            interval="preserveStartEnd"
          />
          <YAxis
            tick={{ fontSize: 9, fontFamily: "monospace", fill: "#64748b" }}
            tickFormatter={(v) => formatMs(v)}
          />
          <Tooltip
            contentStyle={{
              background: "rgba(15,23,42,0.9)",
              border: "1px solid rgba(255,255,255,0.1)",
              borderRadius: 8,
              fontSize: 10,
              fontFamily: "monospace",
            }}
            formatter={(v?: TooltipValueType) => [formatMs(typeof v === 'number' ? v : Number(v ?? 0)), ""]}
          />
          <Legend
            wrapperStyle={{ fontSize: 10, fontFamily: "monospace" }}
          />
          <Line
            type="monotone"
            dataKey="p50"
            stroke="#22c55e"
            strokeWidth={2}
            dot={false}
            name="p50"
          />
          <Line
            type="monotone"
            dataKey="p95"
            stroke="#f97316"
            strokeWidth={2}
            dot={false}
            name="p95"
          />
          <Line
            type="monotone"
            dataKey="p99"
            stroke="#ef4444"
            strokeWidth={2}
            dot={false}
            name="p99"
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

// ── Page ─────────────────────────────────────────────────────────────────────

export default function SettlementDistributionPage() {
  const [data, setData] = useState<SettlementDistributionData | null>(null);
  const [loading, setLoading] = useState(true);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const load = async () => {
    setLoading(true);
    const result = await fetchSettlementDistribution();
    setData(result);
    setLastUpdated(new Date());
    setLoading(false);
  };

  useEffect(() => {
    load();
    const interval = setInterval(load, 5 * 60 * 1000);
    return () => clearInterval(interval);
  }, []);

  if (loading && !data) {
    return (
      <div className="space-y-8 animate-in fade-in duration-500">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {[1, 2, 3].map((i) => <SkeletonCard key={i} />)}
        </div>
        <SkeletonChart height={280} />
      </div>
    );
  }

  if (!data) return null;

  return (
    <ErrorBoundary>
      <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
        {/* Header */}
        <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
          <div>
            <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
              Timing Analysis // #2106
            </div>
            <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
              <Clock className="w-8 h-8 text-accent" />
              Settlement Distribution
            </h2>
          </div>
          <div className="flex items-center gap-3">
            {lastUpdated && (
              <div className="px-4 py-2 glass rounded-lg text-[10px] font-mono uppercase tracking-widest text-muted-foreground">
                Last Sync: {lastUpdated.toLocaleTimeString()}
              </div>
            )}
            <button
              onClick={load}
              className="px-4 py-2 bg-accent text-white rounded-lg text-[10px] font-bold uppercase tracking-widest hover:scale-105 transition-transform flex items-center gap-2"
              aria-label="Refresh settlement distribution data"
            >
              <RefreshCw className={`w-3 h-3 ${loading ? "animate-spin" : ""}`} />
              Refresh
            </button>
          </div>
        </div>

        {/* Network-wide KPIs */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <MetricCard
            label="Network p50"
            value={formatMs(data.network_p50_ms)}
            subLabel="Median settlement time"
          />
          <MetricCard
            label="Network p95"
            value={formatMs(data.network_p95_ms)}
            subLabel="95th percentile"
          />
          <MetricCard
            label="Network p99"
            value={formatMs(data.network_p99_ms)}
            subLabel="99th percentile (tail latency)"
          />
        </div>

        {/* 7-day trend */}
        <TrendChart data={data.trend} />

        {/* Per-corridor breakdown */}
        <div>
          <h3 className="text-xs font-mono font-bold uppercase tracking-widest mb-4 text-muted-foreground">
            Per-Corridor Percentiles
          </h3>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {data.corridors.map((c) => (
              <PercentileRow key={c.corridor_key} corridor={c} />
            ))}
          </div>
        </div>
      </div>
    </ErrorBoundary>
  );
}
