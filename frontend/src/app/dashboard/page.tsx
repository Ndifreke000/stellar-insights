"use client";

import React, { useEffect, useState, useCallback, useMemo } from "react";
import { SkeletonMetricsCard } from "@/components/ui/Skeleton";
import { SettlementSpeedCard } from "@/components/dashboard/SettlementSpeedCard";
import { LiquidityDepthCard } from "@/components/dashboard/LiquidityDepthCard";
import { CorridorHealthCard } from "@/components/dashboard/CorridorHealthCard";
import { TopAssetsCard } from "@/components/dashboard/TopAssetsCard";
import { MetricCard } from "@/components/dashboard/MetricCard";
import {
  CheckCircle2,
  Activity,
  Wallet,
  Clock,
  TrendingUp,
} from "lucide-react";

type Corridor = {
  id: string;
  health: number;
  successRate: number;
};

type TopAsset = {
  asset: string;
  volume: number;
  tvl: number;
};

type TimePoint = {
  ts: string;
  successRate: number;
  settlementMs: number;
  tvl: number;
};

type DashboardData = {
  totalSuccessRate: number;
  activeCorridors: Corridor[];
  topAssets: TopAsset[];
  timeseries: TimePoint[];
};

export default function DashboardPage() {
  const [data, setData] = useState<DashboardData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/dashboard");
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const json = await res.json();
      setData(json);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Failed to load");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
    const id = setInterval(fetchData, 30_000); // refresh every 30s
    return () => clearInterval(id);
  }, [fetchData]);

  const metrics = useMemo(() => {
    if (!data || !data.timeseries || data.timeseries.length === 0) return null;

    const current = data.timeseries[data.timeseries.length - 1];
    const previous = data.timeseries[0]; // Assuming 0 is the start of the period (24h ago)

    // Helper to calculate trend
    const getTrend = (curr: number, prev: number, lowerIsBetter = false) => {
      if (!prev) return undefined;
      const diff = curr - prev;
      const pct = (diff / prev) * 100;

      let direction: "up" | "down" | "neutral" = "neutral";
      if (pct > 0.1) direction = "up";
      if (pct < -0.1) direction = "down";

      // Determine if "Good"
      // Normal: Up is good (Green), Down is bad (Red)
      // LowerIsBetter: Down is good (Green), Up is bad (Red)
      let isGood = true;
      if (lowerIsBetter) {
        isGood = direction === "down" || direction === "neutral";
      } else {
        isGood = direction === "up" || direction === "neutral";
      }

      return {
        value: pct,
        direction,
        isGood,
      };
    };

    return {
      successRate: {
        value: data.totalSuccessRate * 100,
        trend: getTrend(data.totalSuccessRate, previous.successRate, false),
      },
      activeCorridors: {
        value: data.activeCorridors.length,
        // Mock trend for corridors as explicit history isn't in timeseries usually,
        // but we can try if there was a property, defaulting to neutral/mock for now or omitting
        // Omitting trend for corridors to be safe
      },
      tvl: {
        value: current.tvl,
        trend: getTrend(current.tvl, previous.tvl, false),
      },
      settlementTime: {
        value: current.settlementMs,
        trend: getTrend(current.settlementMs, previous.settlementMs, true), // Lower is better
      },
    };
  }, [data]);

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">Network Dashboard</h1>
        <div className="flex gap-2 items-center">
          <button
            className="px-3 py-1 rounded bg-sky-600 text-white text-sm hover:bg-sky-700 transition-colors"
            onClick={() => fetchData()}
            disabled={loading}
          >
            Refresh
          </button>
        </div>
      </div>

      {loading && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {[1, 2, 3, 4].map((i) => (
            <SkeletonMetricsCard key={i} className="" />
          ))}
        </div>
      )}

      {error && (
        <div className="rounded p-4 bg-rose-50 text-rose-700 border border-rose-200">
          Error: {error}
        </div>
      )}

      {data && metrics && (
        <div className="space-y-6">
          {/* Metric Cards Row */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            <MetricCard
              title="Success Rate"
              value={metrics.successRate.value}
              format="percent"
              trend={metrics.successRate.trend}
              icon={CheckCircle2}
              delay={0}
            />
            <MetricCard
              title="Active Corridors"
              value={metrics.activeCorridors.value}
              format="number"
              icon={Activity}
              delay={0.1}
            />
            <MetricCard
              title="Total Liquidity"
              value={metrics.tvl.value}
              format="currency"
              trend={metrics.tvl.trend}
              icon={Wallet}
              delay={0.2}
            />
            <MetricCard
              title="Avg Settlement Time"
              value={metrics.settlementTime.value}
              format="time"
              trend={metrics.settlementTime.trend}
              icon={Clock}
              delay={0.3}
            />
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <SettlementSpeedCard data={data.timeseries} />

            <LiquidityDepthCard data={data.timeseries} />

            <CorridorHealthCard corridors={data.activeCorridors} />

            <TopAssetsCard assets={data.topAssets} />
          </div>
        </div>
      )}
    </div>
  );
}
