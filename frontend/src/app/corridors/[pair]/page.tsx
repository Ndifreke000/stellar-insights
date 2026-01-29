"use client";

import React, { useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import {
  ArrowLeft,
  TrendingUp,
  TrendingDown,
  Zap,
  Droplets,
  CheckCircle2,
  AlertCircle,
  Clock,
  BarChart3,
  ChevronRight,
  Home,
} from "lucide-react";
import {
  getCorridorDetail,
  generateMockCorridorData,
  CorridorDetailData,
  CorridorMetrics,
} from "@/lib/api";
import {
  SuccessRateChart,
  LatencyDistributionChart,
  LiquidityTrendChart,
  VolumeTrendChart,
  SlippageTrendChart,
} from "@/components/corridor-charts";
import { MainLayout } from "@/components/layout";
import Link from "next/link";
import { Skeleton, SkeletonText, SkeletonCard } from "@/components/ui/Skeleton";

export default function CorridorDetailPage() {
  const params = useParams();
  const router = useRouter();
  const corridorPair = params.pair as string;

  const [data, setData] = useState<CorridorDetailData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function fetchData() {
      try {
        setLoading(true);
        // Try to fetch from API first
        try {
          const result = await getCorridorDetail(corridorPair);
          setData(result);
        } catch {
          console.log("API not available, using mock data");
          // Fallback to mock data
          const mockData = generateMockCorridorData(corridorPair);
          setData(mockData);
        }
      } catch (err) {
        setError("Failed to load corridor data");
        console.error(err);
      } finally {
        setLoading(false);
      }
    }

    if (corridorPair) {
      fetchData();
    }
  }, [corridorPair]);

  if (loading) {
    return (
      <MainLayout>
        <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto">
          <div className="mb-6">
            <Skeleton className="h-8 w-48 mb-4" />
            <SkeletonText lines={2} className="max-w-2xl" />
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
            <SkeletonCard />
            <SkeletonCard />
            <SkeletonCard />
            <SkeletonCard />
          </div>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
              <Skeleton className="h-6 w-40 mb-4" />
              <Skeleton className="h-64 w-full" />
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
              <Skeleton className="h-6 w-40 mb-4" />
              <Skeleton className="h-64 w-full" />
            </div>
          </div>
        </div>
      </MainLayout>
    );
  }

  if (error || !data) {
    return (
      <MainLayout>
        <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto">
          <button
            onClick={() => router.push("/corridors")}
            className="flex items-center gap-2 text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 transition-colors font-medium mb-6"
          >
            <ArrowLeft className="w-5 h-5" />
            Back to Corridors
          </button>
          <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-700/50 rounded-lg p-6 text-red-800 dark:text-red-300">
            <AlertCircle className="w-6 h-6 inline mr-2" />
            {error || "Failed to load corridor data"}
          </div>
        </div>
      </MainLayout>
    );
  }

  const corridor = data.corridor;
  const healthColor =
    corridor.health_score >= 90
      ? "text-green-500"
      : corridor.health_score >= 75
        ? "text-yellow-500"
        : "text-red-500";

  const trendIcon =
    corridor.liquidity_trend === "increasing" ? (
      <TrendingUp className="w-5 h-5 text-green-500" />
    ) : corridor.liquidity_trend === "decreasing" ? (
      <TrendingDown className="w-5 h-5 text-red-500" />
    ) : (
      <TrendingUp className="w-5 h-5 text-muted-foreground" />
    );

  return (
    <MainLayout>
      <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto">
        {/* Breadcrumbs */}
        <nav className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400 mb-6 overflow-x-auto whitespace-nowrap pb-2">
          <Link
            href="/dashboard"
            className="flex items-center gap-1 hover:text-blue-600 dark:hover:text-blue-400 transition-colors"
          >
            <Home className="w-4 h-4" />
            Dashboard
          </Link>
          <ChevronRight className="w-4 h-4 text-gray-400 shrink-0" />
          <Link
            href="/corridors"
            className="hover:text-blue-600 dark:hover:text-blue-400 transition-colors"
          >
            Corridors
          </Link>
          <ChevronRight className="w-4 h-4 text-gray-400 shrink-0" />
          <span className="font-semibold text-gray-900 dark:text-white">
            {corridor.source_asset} → {corridor.destination_asset}
          </span>
        </nav>

        {/* Page Header */}
        <div className="mb-8">
          <button
            onClick={() => router.push("/corridors")}
            className="flex items-center gap-2 text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 transition-colors font-medium mb-4 group"
          >
            <ArrowLeft className="w-5 h-5 transition-transform group-hover:-translate-x-1" />
            Back to Corridors
          </button>
          <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
            <div className="min-w-0">
              <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
                {corridor.source_asset} → {corridor.destination_asset}
              </h1>
              <p className="text-gray-600 dark:text-gray-400 mt-1 text-sm font-mono">
                Pair: {corridorPair}
              </p>
            </div>
            <div className="flex items-center gap-4 bg-gray-50 dark:bg-slate-800 p-4 rounded-xl border border-gray-200 dark:border-slate-700">
              <div className="text-right">
                <div className={`text-3xl font-bold ${healthColor}`}>
                  {corridor.health_score.toFixed(1)}
                </div>
                <p className="text-gray-600 dark:text-gray-400 text-xs font-medium uppercase tracking-wider">
                  Health Score
                </p>
              </div>
            </div>
          </div>
        </div>

        {/* Key Metrics Grid */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          {/* Success Rate */}
          <div className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-6 hover:border-blue-500 transition-colors">
            <div className="flex items-center justify-between mb-4">
              <span className="text-gray-600 dark:text-gray-400 text-sm font-medium">
                Success Rate
              </span>
              <CheckCircle2 className="w-5 h-5 text-green-500" />
            </div>
            <div className="text-3xl font-bold text-green-500">
              {corridor.success_rate.toFixed(1)}%
            </div>
            <p className="text-gray-600 dark:text-gray-400 text-xs mt-2">
              {corridor.successful_payments} of {corridor.total_attempts}
            </p>
          </div>

          {/* Average Latency */}
          <div className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-6 hover:border-blue-500 transition-colors">
            <div className="flex items-center justify-between mb-4">
              <span className="text-gray-600 dark:text-gray-400 text-sm font-medium">
                Avg Latency
              </span>
              <Clock className="w-5 h-5 text-blue-500" />
            </div>
            <div className="text-3xl font-bold text-blue-500">
              {corridor.average_latency_ms.toFixed(0)}
              <span className="text-xl">ms</span>
            </div>
            <p className="text-gray-600 dark:text-gray-400 text-xs mt-2">
              Med: {corridor.median_latency_ms}ms | P99:{" "}
              {corridor.p99_latency_ms}ms
            </p>
          </div>

          {/* Liquidity Depth */}
          <div className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-6 hover:border-blue-500 transition-colors">
            <div className="flex items-center justify-between mb-4">
              <span className="text-gray-600 dark:text-gray-400 text-sm font-medium">
                Liquidity Depth
              </span>
              <Droplets className="w-5 h-5 text-purple-500" />
            </div>
            <div className="text-3xl font-bold text-purple-500">
              ${(corridor.liquidity_depth_usd / 1000000).toFixed(2)}M
            </div>
            <div className="flex items-center gap-2 mt-2">
              {trendIcon}
              <p className="text-gray-600 dark:text-gray-400 text-xs capitalize">
                {corridor.liquidity_trend}
              </p>
            </div>
          </div>

          {/* 24h Volume */}
          <div className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-6 hover:border-blue-500 transition-colors">
            <div className="flex items-center justify-between mb-4">
              <span className="text-gray-600 dark:text-gray-400 text-sm font-medium">
                24h Volume
              </span>
              <Zap className="w-5 h-5 text-amber-500" />
            </div>
            <div className="text-3xl font-bold text-amber-500">
              ${(corridor.liquidity_volume_24h_usd / 1000000).toFixed(2)}M
            </div>
            <p className="text-gray-600 dark:text-gray-400 text-xs mt-2">
              {new Date(corridor.last_updated).toLocaleTimeString()}
            </p>
          </div>
        </div>

        {/* Charts Section */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
          {/* Success Rate Chart */}
          <div className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-6 shadow-sm">
            <SuccessRateChart data={data.historical_success_rate} />
          </div>

          {/* Volume Chart */}
          <div className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-6 shadow-sm">
            <VolumeTrendChart data={data.historical_volume} />
          </div>

          {/* Slippage Chart */}
          <div className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-6 shadow-sm">
            <SlippageTrendChart data={data.historical_slippage} />
          </div>

          {/* Latency Distribution */}
          <div className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-6 shadow-sm">
            <LatencyDistributionChart data={data.latency_distribution} />
          </div>

          {/* Liquidity Trends */}
          <div className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-6 shadow-sm lg:col-span-2">
            <LiquidityTrendChart data={data.liquidity_trends} />
          </div>
        </div>

        {/* Related Corridors */}
        {data.related_corridors && data.related_corridors.length > 0 && (
          <div className="mt-8">
            <h2 className="text-2xl font-bold mb-6 flex items-center gap-3">
              <BarChart3 className="w-6 h-6 text-blue-500" />
              Related Corridors
            </h2>
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
              {data.related_corridors.map((related: CorridorMetrics) => (
                <Link
                  key={related.id}
                  href={`/corridors/${related.id}`}
                  className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-4 hover:border-blue-500 hover:shadow-lg transition-all duration-200 transform hover:-translate-y-1 text-left cursor-pointer"
                >
                  <div className="flex justify-between items-start mb-3 gap-2">
                    <div className="min-w-0">
                      <h3 className="font-semibold text-gray-900 dark:text-white">
                        {related.source_asset} → {related.destination_asset}
                      </h3>
                      <p className="text-gray-600 dark:text-gray-400 text-xs truncate">
                        {related.id}
                      </p>
                    </div>
                    <span className="text-green-600 dark:text-green-400 text-sm font-bold shrink-0">
                      {related.success_rate.toFixed(1)}%
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-sm">
                    <div>
                      <p className="text-gray-600 dark:text-gray-400 text-xs">
                        Health
                      </p>
                      <p className="font-semibold text-gray-900 dark:text-white">
                        {related.health_score.toFixed(0)}
                      </p>
                    </div>
                    <div>
                      <p className="text-gray-600 dark:text-gray-400 text-xs">
                        Liquidity
                      </p>
                      <p className="font-semibold text-gray-900 dark:text-white">
                        ${(related.liquidity_depth_usd / 1000000).toFixed(1)}M
                      </p>
                    </div>
                  </div>
                </Link>
              ))}
            </div>
          </div>
        )}

        {/* Footer Info */}
        <div className="mt-8 p-4 bg-gray-50 dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg text-gray-600 dark:text-gray-400 text-sm">
          <p>
            Last updated: {new Date(corridor.last_updated).toLocaleString()}
          </p>
          <p className="mt-2 text-xs">
            Charts update every 5 minutes with 30-day historical data.
          </p>
        </div>
      </div>
    </MainLayout>
  );
}
