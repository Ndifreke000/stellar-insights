"use client";

import React, { useEffect, useState } from "react";
import {
  TrendingUp,
  Activity,
  AlertCircle,
  RefreshCw,
  Download,
} from "lucide-react";
import { Link } from "@/i18n/navigation";
import { fetchAnalyticsMetrics, AnalyticsMetrics } from "@/lib/analytics-api";
import { LiquidityChart } from "@/components/charts/LiquidityChart";
import { TVLChart } from "@/components/charts/TVLChart";
import { SettlementLatencyChart } from "@/components/charts/SettlementLatencyChart";
import { TopCorridors } from "@/components/charts/TopCorridors";
import { LiquidityHeatmap } from "@/components/charts/LiquidityHeatmap";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { Badge } from "@/components/ui/badge";
import { MuxedAccountCard } from "@/components/analytics/MuxedAccountCard";
import { OnChainVerification } from "@/components/OnChainVerification";

export default function AnalyticsPage() {
  const [metrics, setMetrics] = useState<AnalyticsMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  useEffect(() => {
    const loadMetrics = async () => {
      try {
        setLoading(true);
        setError(null);
        const data = await fetchAnalyticsMetrics();
        setMetrics(data);
        setLastUpdated(new Date());
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : "Failed to load metrics";
        setError(errorMessage);

        const isNetworkError = err instanceof TypeError &&
          (err.message.includes('Failed to fetch') ||
            err.message.includes('fetch is not defined') ||
            err.message.includes('Network request failed'));

        if (!isNetworkError) {
          console.error("Error loading analytics metrics:", err);
        }
      } finally {
        setLoading(false);
      }
    };

    loadMetrics();
    const interval = setInterval(loadMetrics, 5 * 60 * 1000);
    return () => clearInterval(interval);
  }, []);

  const handleRefresh = async () => {
    try {
      setLoading(true);
      const data = await fetchAnalyticsMetrics();
      setMetrics(data);
      setLastUpdated(new Date());
    } catch (err) {
      console.error("Error refreshing metrics:", err);
    } finally {
      setLoading(false);
    }
  };

  if (!metrics && loading) {
    return (
      <div className="flex h-[80vh] items-center justify-center">
        <div className="text-sm font-mono text-accent animate-pulse uppercase tracking-widest italic">Calibrating Intelligence Sensors... // 707-Z</div>
      </div>
    );
  }

  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      notation: 'compact',
      maximumFractionDigits: 2
    }).format(value);
  };

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
      {/* Page Header */}
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">Deep Analytics // 03</div>
          <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
            Network Intelligence
          </h2>
        </div>
        <div className="flex items-center gap-3">
          <div className="px-4 py-2 glass rounded-lg text-[10px] font-mono uppercase tracking-widest text-muted-foreground">
            Last Sync: {lastUpdated?.toLocaleTimeString()}
          </div>
          <button
            onClick={handleRefresh}
            className="px-4 py-2 bg-accent text-white rounded-lg text-[10px] font-bold uppercase tracking-widest hover:scale-105 transition-transform flex items-center gap-2"
          >
            <RefreshCw className={`w-3 h-3 ${loading ? "animate-spin" : ""}`} />
            Re-Scan
          </button>
        </div>
      </div>

      {/* Error State */}
      {error && (
        <div className="glass border-red-500/50 p-4 rounded-xl flex items-center justify-between">
          <div className="flex items-center gap-3">
            <AlertCircle className="w-5 h-5 text-red-500" />
            <p className="text-[10px] font-mono text-red-500 uppercase tracking-widest">
              Emergency Shutdown Avoided // Running on Local Cache (Mock Data)
            </p>
          </div>
          <Badge variant="outline" className="border-red-500/30 text-red-500 text-[10px]">FIX_REQUIRED</Badge>
        </div>
      )}

      {/* Key Metrics */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <MetricCard
          label="Cumulative Volume"
          value={metrics ? formatCurrency(metrics.total_volume_usd) : "$0"}
          subLabel="Total Network Flow"
        />
        <MetricCard
          label="Success Probability"
          value={metrics ? `${metrics.avg_success_rate.toFixed(1)}%` : "0%"}
          trend={1.2}
          trendDirection="up"
        />
        <MetricCard
          label="Active Routing Nodes"
          value={metrics ? metrics.active_corridors : "0"}
          subLabel="Online Corridors"
        />
        <MetricCard
          label="Aggregated Liquidity"
          value={metrics ? formatCurrency(metrics.top_corridors.reduce((sum: number, c: any) => sum + c.liquidity_depth_usd, 0)) : "$0"}
          subLabel="Available Capital"
        />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        <div className="lg:col-span-8 space-y-6">
          <div className="glass-card rounded-2xl p-1">
            {metrics && <LiquidityChart data={metrics.liquidity_history} />}
          </div>
          <div className="glass-card rounded-2xl p-1">
            {metrics && <TVLChart data={metrics.tvl_history} />}
          </div>
          <div className="glass-card rounded-2xl p-1">
            {metrics && <SettlementLatencyChart data={metrics.settlement_latency_history} />}
          </div>
        </div>
        <div className="lg:col-span-4 space-y-6">
          <div className="glass-card rounded-2xl p-1">
            {metrics && <TopCorridors corridors={metrics.top_corridors} />}
          </div>
          <div className="glass-card rounded-2xl p-1">
            {metrics && (
              <LiquidityHeatmap
                corridors={metrics.top_corridors}
                onTimePeriodChange={handleRefresh}
              />
            )}
          </div>
          <MuxedAccountCard />
          <OnChainVerification />
        </div>
      </div>
    </div>
  );
}
