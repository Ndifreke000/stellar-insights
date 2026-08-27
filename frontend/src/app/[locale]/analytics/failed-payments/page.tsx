"use client";

import { useEffect, useState } from "react";
import { AlertTriangle, RefreshCw, TrendingDown, Lightbulb, AlertCircle } from "lucide-react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { Badge } from "@/components/ui/badge";
import { SkeletonCard } from "@/components/ui/Skeleton";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import {
  fetchFailedPaymentsAnalysis,
  type FailedPaymentsAnalysis,
  type FailureCategoryBreakdown,
} from "@/lib/analytics-api";

// ── Category colour map ──────────────────────────────────────────────────────
const CATEGORY_COLORS: Record<string, string> = {
  path_not_found: "#f97316",
  insufficient_balance: "#ef4444",
  no_trustline: "#a855f7",
  transaction_failed: "#ec4899",
  offer_crossing: "#eab308",
  timed_out: "#6366f1",
  other: "#64748b",
};

function BreakdownBar({ item }: { item: FailureCategoryBreakdown }) {
  const color = CATEGORY_COLORS[item.category] ?? CATEGORY_COLORS.other;
  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between text-xs font-mono">
        <span className="uppercase tracking-widest text-muted-foreground">{item.label}</span>
        <span className="font-bold" style={{ color }}>
          {item.count.toLocaleString()} · {item.percentage.toFixed(1)}%
        </span>
      </div>
      <div className="h-2 rounded-full bg-white/10 overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-700"
          style={{ width: `${item.percentage}%`, background: color }}
        />
      </div>
      <p className="text-[10px] text-muted-foreground leading-relaxed">{item.recommendation}</p>
    </div>
  );
}

function CorridorFailureRow({
  corridor_key,
  total_failures,
  failure_rate,
  top_category,
}: {
  corridor_key: string;
  total_failures: number;
  failure_rate: number;
  top_category: string;
}) {
  const color = failure_rate > 5 ? "#ef4444" : failure_rate > 3 ? "#f97316" : "#22c55e";
  return (
    <div className="flex items-center justify-between py-2 border-b border-border/30 last:border-0">
      <div>
        <div className="text-xs font-mono font-bold">{corridor_key}</div>
        <div className="text-[10px] text-muted-foreground uppercase tracking-widest">
          {top_category.replace(/_/g, " ")}
        </div>
      </div>
      <div className="text-right">
        <div className="text-xs font-bold" style={{ color }}>
          {failure_rate.toFixed(1)}%
        </div>
        <div className="text-[10px] text-muted-foreground">{total_failures} failed</div>
      </div>
    </div>
  );
}

export default function FailedPaymentsPage() {
  const [data, setData] = useState<FailedPaymentsAnalysis | null>(null);
  const [loading, setLoading] = useState(true);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const load = async () => {
    setLoading(true);
    const result = await fetchFailedPaymentsAnalysis();
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
              Failure Intelligence // #2107
            </div>
            <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
              <TrendingDown className="w-8 h-8 text-red-500" />
              Failed Payment Analysis
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
              aria-label="Refresh failed payments data"
            >
              <RefreshCw className={`w-3 h-3 ${loading ? "animate-spin" : ""}`} />
              Refresh
            </button>
          </div>
        </div>

        {/* KPI Cards */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <MetricCard
            label="Total Failed Payments"
            value={data.total_failed.toLocaleString()}
            subLabel="Last 7 days"
          />
          <MetricCard
            label="Overall Failure Rate"
            value={`${data.overall_failure_rate.toFixed(2)}%`}
            trend={data.overall_failure_rate > 5 ? data.overall_failure_rate : undefined}
            trendDirection={data.overall_failure_rate > 5 ? "up" : "down"}
          />
          <MetricCard
            label="Total Processed"
            value={data.total_processed.toLocaleString()}
            subLabel="Payments analyzed"
          />
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Root Cause Breakdown */}
          <div className="glass-card rounded-2xl p-6 space-y-5">
            <div className="flex items-center gap-2">
              <AlertTriangle className="w-4 h-4 text-orange-500" />
              <h3 className="text-xs font-mono font-bold uppercase tracking-widest">
                Root Cause Breakdown
              </h3>
            </div>
            <div className="space-y-4">
              {data.breakdown.map((item) => (
                <BreakdownBar key={item.category} item={item} />
              ))}
            </div>
          </div>

          {/* Top Failing Corridors */}
          <div className="space-y-6">
            <div className="glass-card rounded-2xl p-6">
              <div className="flex items-center gap-2 mb-4">
                <AlertCircle className="w-4 h-4 text-red-500" />
                <h3 className="text-xs font-mono font-bold uppercase tracking-widest">
                  Top Failing Corridors
                </h3>
              </div>
              <div className="space-y-1">
                {data.top_failing_corridors.length > 0 ? (
                  data.top_failing_corridors.map((c) => (
                    <CorridorFailureRow key={c.corridor_key} {...c} />
                  ))
                ) : (
                  <p className="text-xs text-muted-foreground font-mono">No corridor failures detected.</p>
                )}
              </div>
            </div>

            {/* Actionable Insights */}
            <div className="glass-card rounded-2xl p-6">
              <div className="flex items-center gap-2 mb-4">
                <Lightbulb className="w-4 h-4 text-yellow-400" />
                <h3 className="text-xs font-mono font-bold uppercase tracking-widest">
                  Actionable Insights
                </h3>
              </div>
              <ul className="space-y-3">
                {data.insights.map((insight, i) => (
                  <li key={i} className="flex items-start gap-2">
                    <Badge
                      variant="outline"
                      className="text-[9px] shrink-0 border-accent/40 text-accent mt-0.5"
                    >
                      {String(i + 1).padStart(2, "0")}
                    </Badge>
                    <p className="text-xs text-muted-foreground leading-relaxed">{insight}</p>
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </div>
      </div>
    </ErrorBoundary>
  );
}
