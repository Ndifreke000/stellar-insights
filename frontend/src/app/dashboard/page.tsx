"use client"

import React, { useEffect, useState } from 'react';
import { MetricCard } from '@/components/dashboard/MetricCard';
import { CorridorHealth } from '@/components/dashboard/CorridorHealth';
import { LiquidityChart } from '@/components/dashboard/LiquidityChart';
import { TopAssetsTable } from '@/components/dashboard/TopAssetsTable';
import { SettlementSpeedChart } from '@/components/dashboard/SettlementSpeedChart';
import { Skeleton } from '@/components/ui/Skeleton';
import { useOverviewMetrics } from '@/hooks/useOverviewMetrics';

interface DashboardData {
  kpi: {
    successRate: { value: number; trend: number; trendDirection: 'up' | 'down' };
    activeCorridors: { value: number; trend: number; trendDirection: 'up' | 'down' };
    liquidityDepth: { value: number; trend: number; trendDirection: 'up' | 'down' };
    settlementSpeed: { value: number; trend: number; trendDirection: 'up' | 'down' };
  };
  corridors: any[];
  liquidity: any[];
  assets: any[];
  settlement: any[];
}

const formatCompactNumber = (value: number) =>
  new Intl.NumberFormat(undefined, {
    notation: 'compact',
    maximumFractionDigits: 1,
  }).format(value);

const formatUsdCompact = (value: number) =>
  new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency: 'USD',
    notation: 'compact',
    maximumFractionDigits: 1,
  }).format(value);

const formatUsd = (value: number) =>
  new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency: 'USD',
    maximumFractionDigits: 2,
  }).format(value);

function KpiSkeletonCard({ labelWidthClass }: { labelWidthClass: string }) {
  return (
    <div className="bg-card text-card-foreground rounded-xl border p-6 shadow-sm">
      <Skeleton className={`h-4 ${labelWidthClass} mb-4`} />
      <Skeleton className="h-8 w-28" />
    </div>
  );
}

export default function DashboardPage() {
  const [data, setData] = useState<DashboardData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const {
    data: overview,
    loading: overviewLoading,
    error: overviewError,
  } = useOverviewMetrics();

  useEffect(() => {
    const fetchData = async () => {
      try {
        const response = await fetch('/api/dashboard');
        if (!response.ok) {
          throw new Error('Failed to fetch dashboard data');
        }
        const result = await response.json();
        setData(result);
      } catch (err) {
        // Only log non-network errors to avoid noise
        const isNetworkError = err instanceof TypeError && 
          (err.message.includes('Failed to fetch') || 
           err.message.includes('fetch is not defined') ||
           err.message.includes('Network request failed'));
           
        const errorMessage = err instanceof Error ? err.message : 'An error occurred';
        setError(errorMessage);
        
        if (!isNetworkError) {
          console.error("Dashboard API error:", err);
        }
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, []);

  if (loading) {
    return (
      <div className="flex-1 space-y-4 p-8 pt-6">
        <div className="flex items-center justify-between space-y-2">
          <h2 className="text-3xl font-bold tracking-tight">Network Overview</h2>
        </div>

        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-5">
          <KpiSkeletonCard labelWidthClass="w-36" />
          <KpiSkeletonCard labelWidthClass="w-40" />
          <KpiSkeletonCard labelWidthClass="w-32" />
          <KpiSkeletonCard labelWidthClass="w-44" />
          <KpiSkeletonCard labelWidthClass="w-28" />
        </div>

        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-7">
          <div className="col-span-4">
            <div className="bg-card text-card-foreground rounded-xl border p-6 shadow-sm">
              <Skeleton className="h-5 w-40 mb-6" />
              <Skeleton className="h-64 w-full" />
            </div>
          </div>
          <div className="col-span-3">
            <div className="bg-card text-card-foreground rounded-xl border p-6 shadow-sm">
              <Skeleton className="h-5 w-48 mb-6" />
              <Skeleton className="h-64 w-full" />
            </div>
          </div>
        </div>

        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-7">
          <div className="col-span-3">
            <div className="bg-card text-card-foreground rounded-xl border p-6 shadow-sm">
              <Skeleton className="h-5 w-44 mb-6" />
              <Skeleton className="h-64 w-full" />
            </div>
          </div>
          <div className="col-span-4">
            <div className="bg-card text-card-foreground rounded-xl border p-6 shadow-sm">
              <Skeleton className="h-5 w-36 mb-6" />
              <Skeleton className="h-64 w-full" />
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="text-lg text-red-500">Error: {error}</div>
      </div>
    );
  }

  if (!data) return null;

  return (
    <div className="flex-1 space-y-4 p-8 pt-6">
      <div className="flex items-center justify-between space-y-2">
        <h2 className="text-3xl font-bold tracking-tight">Network Overview</h2>
      </div>

      {/* KPI Cards */}
      {overviewError && (
        <div className="rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-700">
          Unable to load live KPIs: {overviewError}
        </div>
      )}

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-5">
        {overviewLoading ? (
          <>
            <KpiSkeletonCard labelWidthClass="w-36" />
            <KpiSkeletonCard labelWidthClass="w-40" />
            <KpiSkeletonCard labelWidthClass="w-32" />
            <KpiSkeletonCard labelWidthClass="w-44" />
            <KpiSkeletonCard labelWidthClass="w-28" />
          </>
        ) : (
          <>
            <MetricCard
              label="Total Volume"
              value={overview ? formatUsdCompact(overview.total_volume) : "—"}
            />
            <MetricCard
              label="Total Transactions"
              value={
                overview
                  ? formatCompactNumber(overview.total_transactions)
                  : "—"
              }
            />
            <MetricCard
              label="Active Users"
              value={overview ? formatCompactNumber(overview.active_users) : "—"}
            />
            <MetricCard
              label="Avg Transaction Value"
              value={
                overview ? formatUsd(overview.average_transaction_value) : "—"
              }
            />
            <MetricCard
              label="Corridors"
              value={overview ? formatCompactNumber(overview.corridor_count) : "—"}
            />
          </>
        )}
      </div>

      {/* Charts Row 1 */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-7">
        <div className="col-span-4 transition-all duration-300 hover:shadow-md">
          <LiquidityChart data={data.liquidity} />
        </div>
        <div className="col-span-3 transition-all duration-300 hover:shadow-md">
          <CorridorHealth corridors={data.corridors} />
        </div>
      </div>

      {/* Charts Row 2 */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-7">
        <div className="col-span-3 transition-all duration-300 hover:shadow-md">
          <SettlementSpeedChart data={data.settlement} />
        </div>
        <div className="col-span-4 transition-all duration-300 hover:shadow-md">
          <TopAssetsTable assets={data.assets} />
        </div>
      </div>
    </div>
  );
}
