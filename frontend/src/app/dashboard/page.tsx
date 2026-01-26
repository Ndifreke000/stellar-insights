"use client";

import React, { useEffect, useState, useCallback } from "react";
import { SkeletonMetricsCard } from '@/components/ui/Skeleton'
import { KpiCard } from '@/components/dashboard/KpiCard'
import { SettlementSpeedCard } from '@/components/dashboard/SettlementSpeedCard'
import { LiquidityDepthCard } from '@/components/dashboard/LiquidityDepthCard'
import { CorridorHealthCard } from '@/components/dashboard/CorridorHealthCard'
import { TopAssetsCard } from '@/components/dashboard/TopAssetsCard'

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

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">Network Dashboard</h1>
        <div className="flex gap-2 items-center">
          <button
            className="px-3 py-1 rounded bg-sky-600 text-white text-sm"
            onClick={() => fetchData()}
            disabled={loading}
          >
            Refresh
          </button>
        </div>
      </div>

        {loading && (
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <SkeletonMetricsCard className="col-span-1" />
            <SkeletonMetricsCard className="col-span-1 lg:col-span-2" />
            <SkeletonMetricsCard className="col-span-1 lg:col-span-2" />
            <SkeletonMetricsCard className="col-span-1" />
            <SkeletonMetricsCard className="col-span-1 lg:col-span-2" />
          </div>
        )}

      {error && (
        <div className="rounded p-4 bg-rose-50 text-rose-700">
          Error: {error}
        </div>
      )}

      {data && (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          <KpiCard
            title="Total Payment Success Rate"
            value={`${(data.totalSuccessRate * 100).toFixed(2)}%`}
            subtitle="(last 24h)"
          />

          <SettlementSpeedCard data={data.timeseries} />

          <LiquidityDepthCard data={data.timeseries} />

          <CorridorHealthCard corridors={data.activeCorridors} />

          <TopAssetsCard assets={data.topAssets} />
        </div>
      )}
    </div>
  );
}
