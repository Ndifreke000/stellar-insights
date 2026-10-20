"use client";

import { useCallback, useEffect, useState } from "react";
import { AlertCircle, RefreshCw } from "lucide-react";
import { PrintButton } from "@/components/PrintButton";
import {
  getAnalyticsDashboard,
  type AnalyticsDashboardData,
} from "@/lib/analytics";
import { logger } from "@/lib/logger";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { Badge } from "@/components/ui/badge";
import { SkeletonChart, SkeletonCard } from "@/components/ui/Skeleton";
import { MuxedAccountCard } from "@/components/analytics/MuxedAccountCard";
import { OnChainVerification } from "@/components/OnChainVerification";

const formatCurrency = (value: number) => {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    notation: "compact",
    maximumFractionDigits: 2,
  }).format(value);
};

const formatUpdated = (value: Date | null) =>
  value ? value.toLocaleString() : "never";

export default function AnalyticsPage() {
  const [dashboard, setDashboard] = useState<AnalyticsDashboardData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const loadMetrics = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await getAnalyticsDashboard();
      setDashboard(data);
      setLastUpdated(new Date());
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : "Failed to load metrics";
      setError(errorMessage);
      logger.error("Error loading analytics dashboard:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadMetrics();
    const interval = setInterval(() => {
      void loadMetrics();
    }, 5 * 60 * 1000);
    return () => clearInterval(interval);
  }, [loadMetrics]);

  if (!dashboard && loading) {
    return (
      <div className="space-y-8 animate-in fade-in duration-500">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          {[1, 2, 3, 4].map((i) => (
            <SkeletonCard key={i} />
          ))}
        </div>
        <SkeletonChart height={256} />
      </div>
    );
  }

  const observed = Boolean(dashboard?.data_available);
  const series = dashboard?.series ?? dashboard?.time_series_data ?? [];
  const unavailable = Boolean(error) || !observed;

  return (
    <ErrorBoundary>
      <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
        <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
          <div>
            <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
              Deep Analytics // 03
            </div>
            <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
              Network Intelligence
            </h2>
          </div>
          <div className="flex items-center gap-3">
            <div className="px-4 py-2 glass rounded-lg text-[10px] font-mono uppercase tracking-widest text-muted-foreground">
              Last Sync: {lastUpdated?.toLocaleTimeString() ?? "—"}
            </div>
            <button
              onClick={() => {
                void loadMetrics();
              }}
              className="px-4 py-2 bg-accent text-white rounded-lg text-[10px] font-bold uppercase tracking-widest hover:scale-105 transition-transform flex items-center gap-2"
            >
              <RefreshCw className={`w-3 h-3 ${loading ? "animate-spin" : ""}`} />
              Re-Scan
            </button>
            <PrintButton label="Print" />
          </div>
        </div>

        {unavailable && (
          <div
            role="alert"
            className="glass border-red-500/50 p-4 rounded-xl flex items-center justify-between gap-3"
          >
            <div className="flex items-center gap-3">
              <AlertCircle className="w-5 h-5 text-red-500 shrink-0" />
              <p className="text-sm text-red-400">
                Data temporarily unavailable — last updated {formatUpdated(lastUpdated)}
              </p>
            </div>
            <Badge
              variant="outline"
              className="border-red-500/30 text-red-500 text-[10px]"
            >
              OBSERVED_ONLY
            </Badge>
          </div>
        )}

        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <MetricCard
            label="Cumulative Volume"
            value={observed ? formatCurrency(dashboard?.stats.volume_24h ?? 0) : "—"}
            subLabel="Observed 24h flow"
            dataAsOf={lastUpdated}
          />
          <MetricCard
            label="Success Probability"
            value={
              observed
                ? `${(dashboard?.stats.avg_success_rate ?? 0).toFixed(1)}%`
                : "—"
            }
            dataAsOf={lastUpdated}
          />
          <MetricCard
            label="Active Routing Nodes"
            value={observed ? (dashboard?.stats.active_corridors ?? 0) : "—"}
            subLabel="Corridors with payments"
            dataAsOf={lastUpdated}
          />
          <MetricCard
            label="Volume Change"
            value={
              observed ? `${(dashboard?.stats.volume_growth ?? 0).toFixed(1)}%` : "—"
            }
            subLabel="Versus the previous day"
            dataAsOf={lastUpdated}
          />
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-8">
            <div className="glass-card rounded-2xl p-6">
              <h3 className="text-xs font-mono text-muted-foreground uppercase tracking-widest mb-4">
                Observed hourly volume
              </h3>
              {series.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  Data temporarily unavailable — last updated {formatUpdated(lastUpdated)}
                </p>
              ) : (
                <ul className="space-y-2">
                  {series.map((point) => (
                    <li
                      key={point.time}
                      className="flex justify-between text-xs font-mono border-b border-border/30 pb-2"
                    >
                      <span>{point.time}</span>
                      <span>
                        {formatCurrency(point.volume)} · {point.corridors} corridors ·{" "}
                        {point.anchors} anchors
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
          <div className="lg:col-span-4 space-y-6">
            <div className="glass-card rounded-2xl p-6">
              <h3 className="text-xs font-mono text-muted-foreground uppercase tracking-widest mb-4">
                Observed corridors
              </h3>
              {(dashboard?.corridor_performance.length ?? 0) === 0 ? (
                <p className="text-sm text-muted-foreground">No observed corridor rows.</p>
              ) : (
                <ul className="space-y-3">
                  {dashboard?.corridor_performance.map((corridor) => (
                    <li key={corridor.corridor} className="text-xs font-mono">
                      <p className="text-foreground">{corridor.corridor}</p>
                      <p className="text-muted-foreground">
                        {corridor.success_rate.toFixed(1)}% · {formatCurrency(corridor.volume)}
                      </p>
                    </li>
                  ))}
                </ul>
              )}
            </div>
            <MuxedAccountCard />
            <OnChainVerification />
          </div>
        </div>
      </div>
    </ErrorBoundary>
  );
}
