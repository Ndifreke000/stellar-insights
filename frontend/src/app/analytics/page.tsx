"use client";

import React, { useEffect, useState } from "react";
import { MainLayout } from "@/components/layout";
import {
  TrendingUp,
  Activity,
  AlertCircle,
  RefreshCw,
  Download,
} from "lucide-react";
import Link from "next/link";
import { fetchAnalyticsMetrics, AnalyticsMetrics } from "@/lib/analytics-api";
import { LiquidityChart } from "@/components/charts/LiquidityChart";
import { TVLChart } from "@/components/charts/TVLChart";
import { SettlementLatencyChart } from "@/components/charts/SettlementLatencyChart";
import { TopCorridors } from "@/components/charts/TopCorridors";

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
        
        // Only log non-network errors to avoid noise when backend is not running
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

    // Refresh every 5 minutes
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
      // Only log non-network errors to avoid noise when backend is not running
      const isNetworkError = err instanceof TypeError && 
        (err.message.includes('Failed to fetch') || 
         err.message.includes('fetch is not defined') ||
         err.message.includes('Network request failed'));
         
      if (!isNetworkError) {
        console.error("Error refreshing metrics:", err);
      }
    } finally {
      setLoading(false);
    }
  };

  if (!metrics && loading) {
    return (
      <MainLayout>
        <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto">
          <div className="flex items-center justify-center h-96">
            <div className="text-center">
              <RefreshCw className="w-12 h-12 text-gray-400 mx-auto mb-4 animate-spin" />
              <p className="text-gray-600 dark:text-gray-400">
                Loading metrics...
              </p>
            </div>
          </div>
        </div>
      </MainLayout>
    );
  }

  const formatCurrency = (value: number) => {
    if (value >= 1000000) {
      return `$${(value / 1000000).toFixed(2)}M`;
    }
    if (value >= 1000) {
      return `$${(value / 1000).toFixed(0)}K`;
    }
    return `$${value.toFixed(0)}`;
  };

  return (
    <MainLayout>
      <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto">
        {/* Page Header */}
        <div className="mb-8 flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-2">
              Analytics
            </h1>
            <p className="text-gray-600 dark:text-gray-400">
              Deep insights into Stellar network performance and metrics
            </p>
          </div>
          <div className="flex gap-3">
            <Link
              href="/analytics/export"
              className="flex items-center gap-2 px-4 py-2 bg-white dark:bg-slate-800 border border-gray-300 dark:border-slate-600 rounded-lg hover:bg-gray-50 dark:hover:bg-slate-700 transition font-medium text-sm text-gray-700 dark:text-gray-200"
            >
              <Download className="w-4 h-4" />
              Export
            </Link>
            <button
              onClick={handleRefresh}
              disabled={loading}
              className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded-lg transition-colors"
            >
              <RefreshCw
                className={`w-4 h-4 ${loading ? "animate-spin" : ""}`}
              />
              Refresh
            </button>
          </div>
        </div>

        {/* Error State */}
        {error && (
          <div className="mb-8 p-4 bg-red-100 dark:bg-red-900 border border-red-300 dark:border-red-700 rounded-lg">
            <p className="text-red-800 dark:text-red-300 font-medium">
              ⚠️ {error}
            </p>
            <p className="text-sm text-red-700 dark:text-red-400 mt-1">
              Using mock data. Connect the backend API to see real data.
            </p>
          </div>
        )}

        {/* Last Updated */}
        {lastUpdated && (
          <div className="mb-4 text-sm text-gray-600 dark:text-gray-400">
            Last updated: {lastUpdated.toLocaleTimeString()}
          </div>
        )}

        {/* Key Metrics */}
        <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
          <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 bg-blue-100 dark:bg-blue-900 rounded-lg flex items-center justify-center">
                <TrendingUp className="w-6 h-6 text-blue-600 dark:text-blue-300" />
              </div>
              <h3 className="font-medium text-gray-700 dark:text-gray-300">
                Total Volume
              </h3>
            </div>
            <p className="text-2xl font-bold text-gray-900 dark:text-white mb-2">
              {metrics ? formatCurrency(metrics.total_volume_usd) : "$0"}
            </p>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              All corridors
            </p>
          </div>

          <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 bg-green-100 dark:bg-green-900 rounded-lg flex items-center justify-center">
                <Activity className="w-6 h-6 text-green-600 dark:text-green-300" />
              </div>
              <h3 className="font-medium text-gray-700 dark:text-gray-300">
                Avg Success Rate
              </h3>
            </div>
            <p className="text-2xl font-bold text-gray-900 dark:text-white mb-2">
              {metrics ? `${metrics.avg_success_rate.toFixed(1)}%` : "0%"}
            </p>
            <p className="text-sm text-green-600 dark:text-green-400">
              Network-wide
            </p>
          </div>

          <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 bg-yellow-100 dark:bg-yellow-900 rounded-lg flex items-center justify-center">
                <AlertCircle className="w-6 h-6 text-yellow-600 dark:text-yellow-300" />
              </div>
              <h3 className="font-medium text-gray-700 dark:text-gray-300">
                Active Corridors
              </h3>
            </div>
            <p className="text-2xl font-bold text-gray-900 dark:text-white mb-2">
              {metrics ? metrics.active_corridors : "0"}
            </p>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Trading active
            </p>
          </div>

          <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 bg-purple-100 dark:bg-purple-900 rounded-lg flex items-center justify-center">
                <TrendingUp className="w-6 h-6 text-purple-600 dark:text-purple-300" />
              </div>
              <h3 className="font-medium text-gray-700 dark:text-gray-300">
                Total Liquidity
              </h3>
            </div>
            <p className="text-2xl font-bold text-gray-900 dark:text-white mb-2">
              {metrics
                ? formatCurrency(
                    metrics.top_corridors.reduce(
                      (sum, c) => sum + c.liquidity_depth_usd,
                      0,
                    ),
                  )
                : "$0"}
            </p>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Available
            </p>
          </div>
        </div>

        {/* Top Corridors */}
        {metrics && <TopCorridors corridors={metrics.top_corridors} />}

        {/* Charts Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-8 mb-8">
          {metrics && <LiquidityChart data={metrics.liquidity_history} />}
          {metrics && <TVLChart data={metrics.tvl_history} />}
        </div>

        {/* Settlement Latency Chart - Full Width */}
        {metrics && (
          <div className="mb-8">
            <SettlementLatencyChart data={metrics.settlement_latency_history} />
          </div>
        )}
      </div>
    </MainLayout>
  );
}
