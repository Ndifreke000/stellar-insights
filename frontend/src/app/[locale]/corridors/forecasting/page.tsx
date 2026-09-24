"use client";

import React, { useEffect, useState } from "react";
import dynamic from "next/dynamic";
import { Activity, AlertTriangle, RefreshCw } from "lucide-react";
import { getCorridors, CorridorMetrics } from "@/lib/api/corridors";
import { mockCorridors } from "@/components/lib/mockCorridorData";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { SkeletonChart } from "@/components/ui/Skeleton";
import { logger } from "@/lib/logger";

const CorridorForecastingPanel = dynamic(
  () =>
    import("@/components/CorridorForecastingPanel").then((m) => ({
      default: m.CorridorForecastingPanel,
    })),
  { ssr: false, loading: () => <SkeletonChart height={320} /> }
);

export default function CorridorForecastingPage() {
  const [corridors, setCorridors] = useState<CorridorMetrics[]>([]);
  const [loading, setLoading] = useState(true);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isUsingFallback, setIsUsingFallback] = useState(false);

  const loadCorridors = async () => {
    setLoading(true);
    setError(null);
    setIsUsingFallback(false);
    try {
      const data = await getCorridors({ limit: 6, sort_by: "health_score" });
      setCorridors(data);
    } catch (err) {
      logger.warn("Using mock corridor data for forecasting", { error: err });
      setError(err instanceof Error ? err.message : "Failed to load live corridor metrics from backend.");
      setIsUsingFallback(true);
      setCorridors(mockCorridors as unknown as CorridorMetrics[]);
    } finally {
      setLoading(false);
      setLastUpdated(new Date());
    }
  };

  useEffect(() => {
    loadCorridors();
  }, []);

  return (
    <ErrorBoundary>
      <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
        {/* Page header */}
        <div className="flex flex-col sm:flex-row sm:items-start justify-between gap-4">
          <div>
            <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
              Predictive Analytics // 08
            </div>
            <h1 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
              <Activity className="w-8 h-8 text-accent" aria-hidden="true" />
              Corridor Forecasting
            </h1>
          </div>
          <div className="flex items-center gap-3">
            {lastUpdated && (
              <div className="px-4 py-2 glass rounded-lg text-[10px] font-mono uppercase tracking-widest text-muted-foreground">
                Last Sync: {lastUpdated.toLocaleTimeString()}
              </div>
            )}
            <button
              onClick={loadCorridors}
              disabled={loading}
              className="px-4 py-2 bg-accent text-white rounded-lg text-[10px] font-bold uppercase tracking-widest hover:scale-105 transition-transform flex items-center gap-2 disabled:opacity-50"
              aria-label="Refresh forecasts"
            >
              <RefreshCw
                className={`w-3 h-3 ${loading ? "animate-spin" : ""}`}
                aria-hidden="true"
              />
              Re-Scan
            </button>
          </div>
        </div>

        {isUsingFallback && (
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-4 bg-amber-500/10 border border-amber-500/30 rounded-xl text-sm text-amber-400">
            <div className="flex items-center gap-2">
              <AlertTriangle className="w-5 h-5 shrink-0" aria-hidden="true" />
              <span>
                <strong>Live Data Unavailable:</strong> {error || "Could not retrieve live corridor metrics."} Showing offline preview data.
              </span>
            </div>
            <button
              onClick={loadCorridors}
              disabled={loading}
              className="px-3 py-1 bg-amber-500/20 hover:bg-amber-500/30 text-amber-300 rounded-md text-xs font-semibold uppercase tracking-wider transition-colors shrink-0"
            >
              Retry Live Sync
            </button>
          </div>
        )}

        {loading ? (
          <div className="grid grid-cols-1 xl:grid-cols-2 gap-6">
            {[1, 2, 3].map((i) => (
              <SkeletonChart key={i} height={320} />
            ))}
          </div>
        ) : (
          <CorridorForecastingPanel corridors={corridors} maxCorridors={6} />
        )}
      </div>
    </ErrorBoundary>
  );
}
