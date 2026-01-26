"use client";

import React, { useEffect, useState } from "react";
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";
import { MainLayout } from "@/components/layout";
import { TrendingUp, Activity, AlertCircle, Loader2 } from "lucide-react";
import { getAnalyticsDashboard, AnalyticsDashboardData } from "@/lib/analytics";

export default function AnalyticsPage() {
  const [data, setData] = useState<AnalyticsDashboardData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function loadData() {
      try {
        setLoading(true);
        const dashboardData = await getAnalyticsDashboard();
        setData(dashboardData);
        setError(null);
      } catch (err) {
        console.error("Failed to load analytics data:", err);
        setError("Failed to load analytics data. Please try again later.");
      } finally {
        setLoading(false);
      }
    }

    loadData();
  }, []);

  if (loading) {
    return (
      <MainLayout>
        <div className="flex h-[calc(100vh-200px)] items-center justify-center">
          <Loader2 className="h-12 w-12 animate-spin text-blue-500" />
        </div>
      </MainLayout>
    );
  }

  if (error || !data) {
    return (
      <MainLayout>
        <div className="flex h-[calc(100vh-200px)] flex-col items-center justify-center text-center">
          <AlertCircle className="mb-4 h-12 w-12 text-red-500" />
          <h2 className="mb-2 text-xl font-semibold text-gray-900 dark:text-white">
            Error Loading Data
          </h2>
          <p className="text-gray-600 dark:text-gray-400">{error || "No data available"}</p>
          <button 
            onClick={() => window.location.reload()}
            className="mt-4 rounded-lg bg-blue-600 px-4 py-2 text-white hover:bg-blue-700"
          >
            Retry
          </button>
        </div>
      </MainLayout>
    );
  }

  const { stats, timeSeriesData, corridorPerformance } = data;

  return (
    <MainLayout>
      <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto">
        {/* Page Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-2">
            Analytics
          </h1>
          <p className="text-gray-600 dark:text-gray-400">
            Deep insights into Stellar network performance and metrics
          </p>
        </div>

        {/* Key Metrics */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
          <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 bg-blue-100 dark:bg-blue-900 rounded-lg flex items-center justify-center">
                <TrendingUp className="w-6 h-6 text-blue-600 dark:text-blue-300" />
              </div>
              <h3 className="font-medium text-gray-700 dark:text-gray-300">
                Network Volume (24h)
              </h3>
            </div>
            <p className="text-2xl font-bold text-gray-900 dark:text-white mb-2">
              ${(stats.volume24h / 1000000).toFixed(1)}M
            </p>
            <p className="text-sm text-green-600 dark:text-green-400">
              ↑ {stats.volumeGrowth}% from yesterday
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
              {stats.avgSuccessRate}%
            </p>
            <p className="text-sm text-green-600 dark:text-green-400">
              ↑ {stats.successRateGrowth}% from last week
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
              {stats.activeCorridors}
            </p>
            <p className="text-sm text-green-600 dark:text-green-400">
              ↑ {stats.corridorsGrowth} this month
            </p>
          </div>
        </div>

        {/* Charts */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
          {/* Volume & Activity Over Time */}
          <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
            <h2 className="text-lg font-bold text-gray-900 dark:text-white mb-4">
              Network Activity Over Time
            </h2>
            <ResponsiveContainer width="100%" height={300}>
              <LineChart data={timeSeriesData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
                <XAxis dataKey="time" stroke="#6b7280" />
                <YAxis stroke="#6b7280" />
                <Tooltip
                  contentStyle={{
                    backgroundColor: "#1f2937",
                    border: "1px solid #4b5563",
                  }}
                  labelStyle={{ color: "#fff" }}
                />
                <Legend />
                <Line
                  type="monotone"
                  dataKey="volume"
                  stroke="#3b82f6"
                  name="Volume ($)"
                  dot={false}
                />
                <Line
                  type="monotone"
                  dataKey="corridors"
                  stroke="#10b981"
                  name="Corridors"
                  dot={false}
                />
              </LineChart>
            </ResponsiveContainer>
          </div>

          {/* Success Rates by Corridor */}
          <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
            <h2 className="text-lg font-bold text-gray-900 dark:text-white mb-4">
              Success Rate by Corridor
            </h2>
            <ResponsiveContainer width="100%" height={300}>
              <BarChart data={corridorPerformance}>
                <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
                <XAxis dataKey="corridor" stroke="#6b7280" />
                <YAxis stroke="#6b7280" />
                <Tooltip
                  contentStyle={{
                    backgroundColor: "#1f2937",
                    border: "1px solid #4b5563",
                  }}
                  labelStyle={{ color: "#fff" }}
                />
                <Bar
                  dataKey="successRate"
                  fill="#3b82f6"
                  name="Success Rate %"
                />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* Detailed Performance Table */}
        <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
          <h2 className="text-lg font-bold text-gray-900 dark:text-white mb-4">
            Detailed Corridor Performance
          </h2>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-200 dark:border-slate-700">
                  <th className="px-4 py-3 text-left font-medium text-gray-700 dark:text-gray-300">
                    Corridor
                  </th>
                  <th className="px-4 py-3 text-left font-medium text-gray-700 dark:text-gray-300">
                    Success Rate
                  </th>
                  <th className="px-4 py-3 text-left font-medium text-gray-700 dark:text-gray-300">
                    24h Volume
                  </th>
                  <th className="px-4 py-3 text-left font-medium text-gray-700 dark:text-gray-300">
                    Health Score
                  </th>
                  <th className="px-4 py-3 text-left font-medium text-gray-700 dark:text-gray-300">
                    Status
                  </th>
                </tr>
              </thead>
              <tbody>
                {corridorPerformance.map((row, index) => (
                  <tr
                    key={index}
                    className="border-b border-gray-100 dark:border-slate-700 hover:bg-gray-50 dark:hover:bg-slate-700"
                  >
                    <td className="px-4 py-3 text-gray-900 dark:text-white font-medium">
                      {row.corridor}
                    </td>
                    <td className="px-4 py-3 text-gray-700 dark:text-gray-300">
                      {row.successRate}%
                    </td>
                    <td className="px-4 py-3 text-gray-700 dark:text-gray-300">
                      ${(row.volume / 1000).toFixed(0)}K
                    </td>
                    <td className="px-4 py-3">
                      <div className="w-12 bg-gray-200 dark:bg-slate-600 rounded-full h-2">
                        <div
                          className="bg-green-500 h-2 rounded-full"
                          style={{ width: `${row.health}%` }}
                        />
                      </div>
                    </td>
                    <td className="px-4 py-3">
                      <span className="px-3 py-1 bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-300 rounded-full text-xs font-medium">
                        Healthy
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </MainLayout>
  );
}
