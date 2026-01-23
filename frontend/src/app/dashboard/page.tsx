"use client";

import React, { useEffect, useState } from "react";
import { TrendingUp, BarChart3, Anchor, Activity, Loader } from "lucide-react";
import { MainLayout } from "@/components/layout";

interface StatCard {
  title: string;
  value: string | number;
  icon: React.ReactNode;
  description: string;
  trend?: string;
}

export default function DashboardPage() {
  const [stats, setStats] = useState<StatCard[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Simulate fetching dashboard stats
    setTimeout(() => {
      setStats([
        {
          title: "Active Corridors",
          value: "24",
          icon: <TrendingUp className="w-6 h-6" />,
          description: "Payment corridors",
          trend: "+3 this month",
        },
        {
          title: "Network Health",
          value: "98.5%",
          icon: <Activity className="w-6 h-6" />,
          description: "Overall success rate",
          trend: "+2.3% from last week",
        },
        {
          title: "Active Anchors",
          value: "156",
          icon: <Anchor className="w-6 h-6" />,
          description: "Liquidity providers",
          trend: "+12 this month",
        },
        {
          title: "Total Volume",
          value: "$2.4M",
          icon: <BarChart3 className="w-6 h-6" />,
          description: "24h transaction volume",
          trend: "+18% from yesterday",
        },
      ]);
      setLoading(false);
    }, 500);
  }, []);

  return (
    <MainLayout>
      <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto">
        {/* Page Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-2">
            Dashboard
          </h1>
          <p className="text-gray-600 dark:text-gray-400">
            Overview of your Stellar payment network insights
          </p>
        </div>

        {/* Stats Grid */}
        {loading ? (
          <div className="flex items-center justify-center py-12">
            <Loader className="w-8 h-8 animate-spin text-blue-500" />
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
            {stats.map((stat, index) => (
              <div
                key={index}
                className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6 hover:shadow-lg transition-shadow"
              >
                <div className="flex items-start justify-between mb-4">
                  <div className="text-blue-500 dark:text-blue-400">
                    {stat.icon}
                  </div>
                </div>
                <h3 className="text-sm font-medium text-gray-600 dark:text-gray-400 mb-1">
                  {stat.title}
                </h3>
                <p className="text-2xl font-bold text-gray-900 dark:text-white mb-2">
                  {stat.value}
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
                  {stat.description}
                </p>
                {stat.trend && (
                  <p className="text-xs text-green-600 dark:text-green-400">
                    {stat.trend}
                  </p>
                )}
              </div>
            ))}
          </div>
        )}

        {/* Quick Actions */}
        <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
          <h2 className="text-lg font-bold text-gray-900 dark:text-white mb-4">
            Quick Actions
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <a
              href="/corridors"
              className="p-4 border border-gray-200 dark:border-slate-700 rounded-lg hover:bg-gray-50 dark:hover:bg-slate-700 transition-colors"
            >
              <h3 className="font-medium text-gray-900 dark:text-white mb-1">
                Explore Corridors
              </h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                View payment corridors and their performance metrics
              </p>
            </a>
            <a
              href="/anchors"
              className="p-4 border border-gray-200 dark:border-slate-700 rounded-lg hover:bg-gray-50 dark:hover:bg-slate-700 transition-colors"
            >
              <h3 className="font-medium text-gray-900 dark:text-white mb-1">
                Browse Anchors
              </h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Discover liquidity providers and their details
              </p>
            </a>
            <a
              href="/analytics"
              className="p-4 border border-gray-200 dark:border-slate-700 rounded-lg hover:bg-gray-50 dark:hover:bg-slate-700 transition-colors"
            >
              <h3 className="font-medium text-gray-900 dark:text-white mb-1">
                View Analytics
              </h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Deep dive into network performance data
              </p>
            </a>
            <a
              href="/"
              className="p-4 border border-gray-200 dark:border-slate-700 rounded-lg hover:bg-gray-50 dark:hover:bg-slate-700 transition-colors"
            >
              <h3 className="font-medium text-gray-900 dark:text-white mb-1">
                Return to Home
              </h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Go back to the landing page
              </p>
            </a>
          </div>
        </div>
      </div>
    </MainLayout>
  );
}