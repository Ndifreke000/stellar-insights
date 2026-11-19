"use client";

import React, { useCallback, useEffect, useState } from "react";
import { Waves, Droplets, Activity, AlertCircle } from "lucide-react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { LiquidityChart } from "@/components/charts/LiquidityChart";
import { Badge } from "@/components/ui/badge";
import { SkeletonChart, SkeletonCard } from "@/components/ui/Skeleton";
import {
  fetchObservedLiquidity,
  type ObservedLiquidity,
} from "@/lib/liquidity-pool-api";

const formatCurrency = (value: number) =>
  new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    notation: "compact",
    maximumFractionDigits: 2,
  }).format(value);

export default function LiquidityPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [observed, setObserved] = useState<ObservedLiquidity | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setObserved(await fetchObservedLiquidity());
    } catch (err) {
      setObserved(null);
      setError(err instanceof Error ? err.message : "Failed to load liquidity pools");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (loading && !observed) {
    return (
      <div className="space-y-8">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {[1, 2, 3].map((item) => (
            <SkeletonCard key={item} />
          ))}
        </div>
        <SkeletonChart height={320} />
      </div>
    );
  }

  const pools = observed?.pools ?? [];
  const points = observed?.points ?? [];
  const stats = observed?.stats ?? null;
  const empty = !error && pools.length === 0;

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
            Market Dynamics // 05
          </div>
          <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
            <Waves className="w-8 h-8 text-accent" />
            Liquidity Terminal
          </h2>
        </div>
        <div className="flex items-center gap-3">
          <Badge variant="outline" className="text-[10px] font-mono border-border/50 px-3 py-1 bg-accent/5">
            OBSERVED_POOLS
          </Badge>
          <button
            type="button"
            onClick={() => {
              void load();
            }}
            className="px-3 py-1 text-[10px] font-bold uppercase tracking-widest border border-border/50 rounded-lg"
          >
            Retry
          </button>
        </div>
      </div>

      {error && (
        <div role="alert" className="flex items-start gap-2 p-4 rounded-xl border border-red-500/30 bg-red-500/10 text-red-300 text-sm">
          <AlertCircle className="w-5 h-5 shrink-0" aria-hidden="true" />
          <p>Liquidity data is unavailable. {error}</p>
        </div>
      )}

      {observed?.partialWarning && (
        <div role="status" className="p-4 rounded-xl border border-amber-500/30 bg-amber-500/10 text-amber-300 text-sm">
          {observed.partialWarning}
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <MetricCard
          label="Total Liquidity Depth"
          value={stats ? formatCurrency(stats.total_value_locked_usd) : "—"}
          subLabel="Observed pool value"
        />
        <MetricCard
          label="24h Pool Volume"
          value={stats ? formatCurrency(stats.total_volume_24h_usd) : "—"}
          subLabel="Observed pool volume"
        />
        <MetricCard
          label="Tracked Pools"
          value={stats ? stats.total_pools : "—"}
          subLabel="Pools returned by the API"
        />
      </div>

      <div className="glass-card rounded-2xl p-1">
        <div className="p-6 pb-2">
          <h3 className="text-xs font-mono text-muted-foreground uppercase tracking-widest mb-4 flex items-center gap-2">
            <Activity className="w-3 h-3 text-accent" />
            Observed liquidity
          </h3>
        </div>
        {empty || points.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <Droplets className="w-12 h-12 text-muted-foreground/30 mb-4" />
            <p className="text-sm font-mono text-muted-foreground uppercase tracking-widest">
              {empty
                ? "No liquidity pools observed"
                : "No liquidity history observed for the loaded pools"}
            </p>
          </div>
        ) : (
          <LiquidityChart data={points} />
        )}
      </div>
    </div>
  );
}
