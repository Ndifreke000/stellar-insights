"use client";

import React from "react";
import { useOverviewMetrics } from "@/hooks/useOverviewMetrics";
import { SkeletonDashboard } from "@/components/ui/Skeleton";
import { LiquidityDepthCard } from "@/components/dashboard/LiquidityDepthCard";
import { CorridorHealthCard } from "@/components/dashboard/CorridorHealthCard";
import { TopAssetsCard } from "@/components/dashboard/TopAssetsCard";
import { SettlementSpeedCard } from "@/components/dashboard/SettlementSpeedCard";
import { MetricCard } from "@/components/dashboard/MetricCard";
import {
  CheckCircle2,
  Activity,
  Wallet,
  Clock,
  RefreshCw,
  AlertCircle,
} from "lucide-react";

function ErrorFallback({
  error,
  onRetry,
}: {
  error: Error;
  onRetry: () => void;
}) {
  return (
    <div className="rounded-lg p-6 bg-rose-50 dark:bg-rose-900/20 border border-rose-200 dark:border-rose-800">
      <div className="flex items-start gap-3">
        <AlertCircle className="w-5 h-5 text-rose-600 dark:text-rose-400 mt-0.5 flex-shrink-0" />
        <div className="flex-1">
          <h3 className="font-medium text-rose-800 dark:text-rose-200">
            Failed to load dashboard data
          </h3>
          <p className="text-sm text-rose-600 dark:text-rose-300 mt-1">
            {error.message || "An unexpected error occurred"}
          </p>
          <button
            onClick={onRetry}
            className="mt-4 inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-rose-600 hover:bg-rose-700 text-white text-sm font-medium transition-colors"
          >
            <RefreshCw className="w-4 h-4" />
            Try Again
          </button>
        </div>
      </div>
    </div>
  );
}

function DashboardContent() {
  const { data, metrics, isLoading, isFetching, isError, error, refetch } =
    useOverviewMetrics();

  // Loading state with skeleton UI
  if (isLoading) {
    return (
      <div className="p-6 space-y-6">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold">Network Dashboard</h1>
          <button
            className="px-3 py-1 rounded bg-sky-600 text-white text-sm hover:bg-sky-700 transition-colors opacity-50 cursor-not-allowed"
            disabled
          >
            Loading...
          </button>
        </div>
        <SkeletonDashboard />
      </div>
    );
  }

  // Error state with retry option
  if (isError && error) {
    return (
      <div className="p-6 space-y-6">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold">Network Dashboard</h1>
        </div>
        <ErrorFallback error={error} onRetry={() => refetch()} />
      </div>
    );
  }

  // No data state
  if (!data || !metrics) {
    return (
      <div className="p-6 space-y-6">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold">Network Dashboard</h1>
          <button
            className="px-3 py-1 rounded bg-sky-600 text-white text-sm hover:bg-sky-700 transition-colors"
            onClick={() => refetch()}
          >
            Refresh
          </button>
        </div>
        <div className="rounded-lg p-6 bg-gray-50 dark:bg-slate-800 border border-gray-200 dark:border-slate-700 text-center">
          <p className="text-gray-600 dark:text-gray-400">
            No dashboard data available
          </p>
        </div>
      </div>
    );
  }

  // Success state with live data
  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">Network Dashboard</h1>
        <div className="flex gap-2 items-center">
          {isFetching && (
            <span className="text-sm text-gray-500 flex items-center gap-1">
              <RefreshCw className="w-3 h-3 animate-spin" />
              Updating...
            </span>
          )}
          <button
            className="px-3 py-1 rounded bg-sky-600 text-white text-sm hover:bg-sky-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1"
            onClick={() => refetch()}
            disabled={isFetching}
          >
            <RefreshCw className={`w-3 h-3 ${isFetching ? "animate-spin" : ""}`} />
            Refresh
          </button>
        </div>
      </div>

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
    </div>
  );
}

export default function DashboardPage() {
  return <DashboardContent />;
}
