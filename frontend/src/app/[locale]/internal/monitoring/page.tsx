"use client";

import React, { useState, useEffect } from "react";
import { MainLayout } from "@/components/layout";
import {
  Activity,
  AlertTriangle,
  Clock,
  Globe,
  Monitor,
  RefreshCw,
  Zap,
} from "lucide-react";
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
  AreaChart,
  Area,
} from "recharts";
import { getMockMonitoringStats, MonitoringStats } from "@/lib/mock-monitoring";

const COLORS = ["#3b82f6", "#10b981", "#f59e0b", "#8b5cf6", "#ec4899"];

export default function MonitoringDashboard() {
  const [stats, setStats] = useState<MonitoringStats | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchData = () => {
      setLoading(true);
      // Simulate API latency
      setTimeout(() => {
        setStats(getMockMonitoringStats());
        setLoading(false);
      }, 800);
    };

    fetchData();
  }, []);

  if (loading || !stats) {
    return (
      <MainLayout>
        <div className="flex items-center justify-center h-screen">
          <RefreshCw className="w-8 h-8 text-blue-500 animate-spin" />
        </div>
      </MainLayout>
    );
  }

  return (
    <MainLayout>
      <div className="p-6 max-w-7xl mx-auto">
        <header className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-2 flex items-center gap-2">
            <Activity className="text-blue-500" />
            Internal Performance Dashboard
          </h1>
          <p className="text-gray-600 dark:text-gray-400">
            Real-time monitoring of frontend vitals, API health, and system
            errors.
          </p>
        </header>

        {/* KPI Cards */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
          <div className="bg-white dark:bg-slate-800 p-6 rounded-xl border border-gray-200 dark:border-slate-700 shadow-sm">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Total Sessions
              </h3>
              <Globe className="text-blue-500 w-5 h-5" />
            </div>
            <p className="text-3xl font-bold dark:text-white">
              {stats.sessionStats.totalSessions.toLocaleString()}
            </p>
            <p className="text-sm text-green-500 mt-2 flex items-center gap-1">
              <span>↑ 12%</span>
              <span className="text-gray-400 font-normal">vs last 24h</span>
            </p>
          </div>

          <div className="bg-white dark:bg-slate-800 p-6 rounded-xl border border-gray-200 dark:border-slate-700 shadow-sm">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Avg. Session Duration
              </h3>
              <Clock className="text-purple-500 w-5 h-5" />
            </div>
            <p className="text-3xl font-bold dark:text-white">
              {Math.floor(stats.sessionStats.avgDuration / 60)}m{" "}
              {stats.sessionStats.avgDuration % 60}s
            </p>
            <p className="text-sm text-gray-400 mt-2">Target: 5m 00s</p>
          </div>

          <div className="bg-white dark:bg-slate-800 p-6 rounded-xl border border-gray-200 dark:border-slate-700 shadow-sm">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Bounce Rate
              </h3>
              <Zap className="text-yellow-500 w-5 h-5" />
            </div>
            <p className="text-3xl font-bold dark:text-white">
              {stats.sessionStats.bounceRate}%
            </p>
            <p className="text-sm text-red-500 mt-2 flex items-center gap-1">
              <span>↑ 2.1%</span>
              <span className="text-gray-400 font-normal">vs last 24h</span>
            </p>
          </div>
        </div>

        {/* Charts Section */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-8">
          {/* Page Load Times */}
          <div className="bg-white dark:bg-slate-800 p-6 rounded-xl border border-gray-200 dark:border-slate-700 shadow-sm">
            <h3 className="text-lg font-semibold mb-6 dark:text-white flex items-center gap-2">
              <Clock size={18} className="text-blue-500" />
              Page Load Times (Seconds)
            </h3>
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={stats.pageLoadTimes}>
                  <defs>
                    <linearGradient id="colorValue" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.8} />
                      <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid
                    strokeDasharray="3 3"
                    vertical={false}
                    stroke="#374151"
                    opacity={0.1}
                  />
                  <XAxis
                    dataKey="timestamp"
                    stroke="#94a3b8"
                    fontSize={12}
                    tickLine={false}
                    axisLine={false}
                  />
                  <YAxis
                    stroke="#94a3b8"
                    fontSize={12}
                    tickLine={false}
                    axisLine={false}
                  />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: "#1e293b",
                      border: "none",
                      borderRadius: "8px",
                      color: "#fff",
                    }}
                    itemStyle={{ color: "#60a5fa" }}
                  />
                  <Area
                    type="monotone"
                    dataKey="value"
                    stroke="#3b82f6"
                    fillOpacity={1}
                    fill="url(#colorValue)"
                  />
                </AreaChart>
              </ResponsiveContainer>
            </div>
          </div>

          {/* Error Rate */}
          <div className="bg-white dark:bg-slate-800 p-6 rounded-xl border border-gray-200 dark:border-slate-700 shadow-sm">
            <h3 className="text-lg font-semibold mb-6 dark:text-white flex items-center gap-2">
              <AlertTriangle size={18} className="text-red-500" />
              Error Frequency
            </h3>
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={stats.errorRates}>
                  <CartesianGrid
                    strokeDasharray="3 3"
                    vertical={false}
                    stroke="#374151"
                    opacity={0.1}
                  />
                  <XAxis
                    dataKey="timestamp"
                    stroke="#94a3b8"
                    fontSize={12}
                    tickLine={false}
                    axisLine={false}
                  />
                  <YAxis
                    stroke="#94a3b8"
                    fontSize={12}
                    tickLine={false}
                    axisLine={false}
                  />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: "#1e293b",
                      border: "none",
                      borderRadius: "8px",
                      color: "#fff",
                    }}
                    cursor={{ fill: "#334155", opacity: 0.4 }}
                  />
                  <Bar dataKey="count" fill="#ef4444" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8 mb-8">
          {/* API Latency */}
          <div className="lg:col-span-2 bg-white dark:bg-slate-800 p-6 rounded-xl border border-gray-200 dark:border-slate-700 shadow-sm">
            <h3 className="text-lg font-semibold mb-6 dark:text-white flex items-center gap-2">
              <Zap size={18} className="text-yellow-500" />
              API Response Times (ms)
            </h3>
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart
                  layout="vertical"
                  data={stats.apiLatencies}
                  margin={{ left: 40 }}
                >
                  <CartesianGrid
                    strokeDasharray="3 3"
                    horizontal={false}
                    stroke="#374151"
                    opacity={0.1}
                  />
                  <XAxis
                    type="number"
                    stroke="#94a3b8"
                    fontSize={12}
                    tickLine={false}
                    axisLine={false}
                  />
                  <YAxis
                    type="category"
                    dataKey="endpoint"
                    stroke="#94a3b8"
                    fontSize={10}
                    tickLine={false}
                    axisLine={false}
                    width={80}
                  />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: "#1e293b",
                      border: "none",
                      borderRadius: "8px",
                      color: "#fff",
                    }}
                  />
                  <Bar dataKey="latency" fill="#f59e0b" radius={[0, 4, 4, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </div>

          {/* Browser / Device breakdown */}
          <div className="bg-white dark:bg-slate-800 p-6 rounded-xl border border-gray-200 dark:border-slate-700 shadow-sm">
            <h3 className="text-lg font-semibold mb-6 dark:text-white flex items-center gap-2">
              <Monitor size={18} className="text-purple-500" />
              Browser Breakdown
            </h3>
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={stats.browserBreakdown}
                    innerRadius={60}
                    outerRadius={80}
                    paddingAngle={5}
                    dataKey="value"
                  >
                    {stats.browserBreakdown.map((entry, index) => (
                      <Cell
                        key={`cell-${index}`}
                        fill={COLORS[index % COLORS.length]}
                      />
                    ))}
                  </Pie>
                  <Tooltip
                    contentStyle={{
                      backgroundColor: "#1e293b",
                      border: "none",
                      borderRadius: "8px",
                      color: "#fff",
                    }}
                  />
                </PieChart>
              </ResponsiveContainer>
            </div>
            <div className="mt-4 flex flex-wrap justify-center gap-4">
              {stats.browserBreakdown.map((entry, index) => (
                <div key={entry.name} className="flex items-center gap-1">
                  <div
                    className="w-3 h-3 rounded-full"
                    style={{ backgroundColor: COLORS[index % COLORS.length] }}
                  ></div>
                  <span className="text-xs text-gray-500">{entry.name}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </MainLayout>
  );
}
