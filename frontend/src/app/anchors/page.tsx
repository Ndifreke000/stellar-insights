"use client";

import React, { useState, useEffect, useMemo, Suspense } from "react";
import {
  Anchor,
  Search,
  TrendingUp,
  TrendingDown,
  AlertCircle,
  CheckCircle,
  Activity,
  Loader,
  ExternalLink,
  BarChart3,
} from "lucide-react";
import { MainLayout } from "@/components/layout";
import { AnchorMetrics } from "@/lib/api";
import Link from "next/link";
import { LineChart, Line, ResponsiveContainer } from "recharts";
import { usePagination } from "@/hooks/usePagination";
import { DataTablePagination } from "@/components/ui/DataTablePagination";
import { SkeletonTable } from "@/components/ui/Skeleton";

// Mock data for demonstration
const generateMockAnchors = (): AnchorMetrics[] => [
  {
    id: "GCKFBEIYTKP6RCZX6DSQT4JDKQF6NKPZ7IQXQJY5QJZQJZQJZQJZQJZQ",
    name: "Circle USDC Anchor",
    stellar_account: "GCKFBEIYTKP6RCZX6DSQT4JDKQF6NKPZ7IQXQJY5QJZQJZQJZQJZQJZQ",
    reliability_score: 98.5,
    asset_coverage: 3,
    failure_rate: 1.5,
    total_transactions: 15420,
    successful_transactions: 15188,
    failed_transactions: 232,
    status: "Healthy",
  },
  {
    id: "GDQOE23CFSUMSVQK4Y5JHPPYK73VYCNHZHA7ENKCV37P6SUEO6XQBKPP",
    name: "MoneyGram Access",
    stellar_account: "GDQOE23CFSUMSVQK4Y5JHPPYK73VYCNHZHA7ENKCV37P6SUEO6XQBKPP",
    reliability_score: 94.2,
    asset_coverage: 5,
    failure_rate: 5.8,
    total_transactions: 8750,
    successful_transactions: 8242,
    failed_transactions: 508,
    status: "Healthy",
  },
  {
    id: "GATEMHCCKCY67ZUCKTROYN24ZYT5GK4EQZ65JJLDHKHRUZI3EUEKMTCH",
    name: "AnchorUSD",
    stellar_account: "GATEMHCCKCY67ZUCKTROYN24ZYT5GK4EQZ65JJLDHKHRUZI3EUEKMTCH",
    reliability_score: 91.8,
    asset_coverage: 2,
    failure_rate: 8.2,
    total_transactions: 5230,
    successful_transactions: 4801,
    failed_transactions: 429,
    status: "Warning",
  },
  {
    id: "GBSTRUSD7IRX73RQZBL3RQUH6KS3O4NYFY3QCALDLZD77XMZOPWAVTUK",
    name: "Stellar Development Foundation",
    stellar_account: "GBSTRUSD7IRX73RQZBL3RQUH6KS3O4NYFY3QCALDLZD77XMZOPWAVTUK",
    reliability_score: 96.7,
    asset_coverage: 4,
    failure_rate: 3.3,
    total_transactions: 12100,
    successful_transactions: 11700,
    failed_transactions: 400,
    status: "Healthy",
  },
  {
    id: "GCKFBEIYTKP6RCZX6DSQT4JDKQF6NKPZ7IQXQJY5QJZQJZQJZQJZQJZA",
    name: "Vibrant Network",
    stellar_account: "GCKFBEIYTKP6RCZX6DSQT4JDKQF6NKPZ7IQXQJY5QJZQJZQJZQJZQJZA",
    reliability_score: 87.3,
    asset_coverage: 6,
    failure_rate: 12.7,
    total_transactions: 3420,
    successful_transactions: 2986,
    failed_transactions: 434,
    status: "Warning",
  },
  {
    id: "GATEMHCCKCY67ZUCKTROYN24ZYT5GK4EQZ65JJLDHKHRUZI3EUEKMTCZ",
    name: "Tempo Money Transfer",
    stellar_account: "GATEMHCCKCY67ZUCKTROYN24ZYT5GK4EQZ65JJLDHKHRUZI3EUEKMTCZ",
    reliability_score: 82.1,
    asset_coverage: 8,
    failure_rate: 17.9,
    total_transactions: 2150,
    successful_transactions: 1765,
    failed_transactions: 385,
    status: "Critical",
  },
];

// Generate mock historical data for mini charts
const generateMockHistoricalData = (baseScore: number) => {
  const data = [];
  const now = new Date();

  for (let i = 29; i >= 0; i--) {
    const date = new Date(now.getTime() - i * 24 * 60 * 60 * 1000);
    // Use deterministic variation based on date to avoid Math.random during render
    const variation = (((date.getTime() / 1000) % 20) - 10);
    data.push({
      date: date.toISOString().split("T")[0],
      score: Math.max(0, Math.min(100, baseScore + variation)),
    });
  }

  return data;
};

function AnchorsPageContent() {
  const [anchors, setAnchors] = useState<AnchorMetrics[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState("");
  const [sortBy, setSortBy] = useState<
    "reliability" | "transactions" | "failure_rate"
  >("reliability");
  const [sortOrder, setSortOrder] = useState<"asc" | "desc">("desc");

  const filteredAndSortedAnchors = useMemo(() => {
    return anchors
      .filter(
        (anchor) =>
          anchor.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
          anchor.stellar_account.toLowerCase().includes(searchTerm.toLowerCase()),
      )
      .sort((a, b) => {
        let aValue, bValue;

        switch (sortBy) {
          case "reliability":
            aValue = a.reliability_score;
            bValue = b.reliability_score;
            break;
          case "transactions":
            aValue = a.total_transactions;
            bValue = b.total_transactions;
            break;
          case "failure_rate":
            aValue = a.failure_rate;
            bValue = b.failure_rate;
            break;
          default:
            aValue = a.reliability_score;
            bValue = b.reliability_score;
        }

        return sortOrder === "desc" ? bValue - aValue : aValue - bValue;
      });
  }, [anchors, searchTerm, sortBy, sortOrder]);

  const {
    currentPage,
    pageSize,
    onPageChange,
    onPageSizeChange,
    startIndex,
    endIndex,
  } = usePagination(filteredAndSortedAnchors.length);

  useEffect(() => {
    const fetchAnchors = async () => {
      try {
        // Try to fetch from API, fallback to mock data
        // const response = await getAnchors();
        // setAnchors(response.anchors);

        // For now, use mock data
        setTimeout(() => {
          setAnchors(generateMockAnchors());
          setLoading(false);
        }, 800);
      } catch (error) {
        console.error("Failed to fetch anchors:", error);
        // Fallback to mock data
        setAnchors(generateMockAnchors());
        setLoading(false);
      }
    };

    fetchAnchors();
  }, []);

  const paginatedAnchors = filteredAndSortedAnchors.slice(startIndex, endIndex);


  const getHealthStatusColor = (status: string) => {
    switch (status.toLowerCase()) {
      case "healthy":
        return "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300";
      case "warning":
        return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-300";
      case "critical":
        return "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300";
      default:
        return "bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-300";
    }
  };

  const getHealthStatusIcon = (status: string) => {
    switch (status.toLowerCase()) {
      case "healthy":
        return <CheckCircle className="w-4 h-4" />;
      case "warning":
        return <AlertCircle className="w-4 h-4" />;
      case "critical":
        return <AlertCircle className="w-4 h-4" />;
      default:
        return <Activity className="w-4 h-4" />;
    }
  };

  const formatNumber = (num: number) => {
    if (num >= 1000000) {
      return `${(num / 1000000).toFixed(1)}M`;
    } else if (num >= 1000) {
      return `${(num / 1000).toFixed(1)}K`;
    }
    return num.toString();
  };

  const truncateAddress = (address: string) => {
    return `${address.slice(0, 8)}...${address.slice(-8)}`;
  };

  return (
    <MainLayout>
      <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto">
        {/* Page Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-2">
            Anchor Analytics
          </h1>
          <p className="text-gray-600 dark:text-gray-400">
            Monitor anchor reliability, asset coverage, and transaction success
            rates
          </p>
        </div>

        {/* Controls */}
        <div className="mb-6 flex flex-col sm:flex-row gap-4">
          {/* Search Bar */}
          <div className="flex-1">
            <div className="relative">
              <Search className="absolute left-4 top-3 w-5 h-5 text-gray-400" />
              <input
                type="text"
                placeholder="Search anchors by name or address..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="w-full pl-10 pr-4 py-2 border border-gray-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>

          {/* Sort Controls */}
          <div className="flex gap-2">
            <select
              value={sortBy}
              onChange={(e) =>
                setSortBy(
                  e.target.value as
                  | "reliability"
                  | "transactions"
                  | "failure_rate",
                )
              }
              className="px-3 py-2 border border-gray-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="reliability">Reliability Score</option>
              <option value="transactions">Total Transactions</option>
              <option value="failure_rate">Failure Rate</option>
            </select>

            <button
              onClick={() =>
                setSortOrder(sortOrder === "desc" ? "asc" : "desc")
              }
              className="px-3 py-2 border border-gray-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white hover:bg-gray-50 dark:hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              {sortOrder === "desc" ? (
                <TrendingDown className="w-4 h-4" />
              ) : (
                <TrendingUp className="w-4 h-4" />
              )}
            </button>
          </div>
        </div>

        {/* Anchors Table */}
        {loading ? (
          <SkeletonTable rows={10} />
        ) : filteredAndSortedAnchors.length === 0 ? (
          <div className="text-center py-12">
            <Anchor className="w-12 h-12 text-gray-400 mx-auto mb-4" />
            <p className="text-gray-600 dark:text-gray-400">
              No anchors found matching your search.
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 overflow-hidden">
              {/* Desktop Table */}
              <div className="hidden lg:block overflow-x-auto">
                <table className="w-full">
                  <thead className="bg-gray-50 dark:bg-slate-700">
                    <tr>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Anchor
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Health Status
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Reliability Score
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Success Rate
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Asset Coverage
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Total Transactions
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        30-Day Trend
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Actions
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-200 dark:divide-slate-700">
                    {paginatedAnchors.map((anchor) => {
                      const successRate =
                        (anchor.successful_transactions /
                          anchor.total_transactions) *
                        100;
                      const historicalData = generateMockHistoricalData(
                        anchor.reliability_score,
                      );

                      return (
                        <tr
                          key={anchor.id}
                          className="hover:bg-gray-50 dark:hover:bg-slate-700"
                        >
                          <td className="px-6 py-4 whitespace-nowrap">
                            <div className="flex items-center">
                              <div className="w-10 h-10 bg-blue-100 dark:bg-blue-900 rounded-lg flex items-center justify-center mr-3">
                                <Anchor className="w-5 h-5 text-blue-600 dark:text-blue-300" />
                              </div>
                              <div>
                                <div className="text-sm font-medium text-gray-900 dark:text-white">
                                  {anchor.name}
                                </div>
                                <div className="text-xs text-gray-500 dark:text-gray-400 font-mono">
                                  {truncateAddress(anchor.stellar_account)}
                                </div>
                              </div>
                            </div>
                          </td>
                          <td className="px-6 py-4 whitespace-nowrap">
                            <span
                              className={`inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium ${getHealthStatusColor(anchor.status)}`}
                            >
                              {getHealthStatusIcon(anchor.status)}
                              {anchor.status}
                            </span>
                          </td>
                          <td className="px-6 py-4 whitespace-nowrap">
                            <div className="flex items-center">
                              <div className="text-sm font-medium text-gray-900 dark:text-white">
                                {anchor.reliability_score.toFixed(1)}%
                              </div>
                              <div className="ml-2 w-16 bg-gray-200 dark:bg-slate-600 rounded-full h-2">
                                <div
                                  className={`h-2 rounded-full ${anchor.reliability_score >= 95
                                      ? "bg-green-500"
                                      : anchor.reliability_score >= 85
                                        ? "bg-yellow-500"
                                        : "bg-red-500"
                                    }`}
                                  style={{
                                    width: `${anchor.reliability_score}%`,
                                  }}
                                />
                              </div>
                            </div>
                          </td>
                          <td className="px-6 py-4 whitespace-nowrap">
                            <div className="text-sm text-gray-900 dark:text-white">
                              {successRate.toFixed(1)}%
                            </div>
                            <div className="text-xs text-gray-500 dark:text-gray-400">
                              {formatNumber(anchor.successful_transactions)}/
                              {formatNumber(anchor.total_transactions)}
                            </div>
                          </td>
                          <td className="px-6 py-4 whitespace-nowrap">
                            <div className="text-sm font-medium text-gray-900 dark:text-white">
                              {anchor.asset_coverage} assets
                            </div>
                          </td>
                          <td className="px-6 py-4 whitespace-nowrap">
                            <div className="text-sm font-medium text-gray-900 dark:text-white">
                              {formatNumber(anchor.total_transactions)}
                            </div>
                            <div className="text-xs text-red-500">
                              {formatNumber(anchor.failed_transactions)} failed
                            </div>
                          </td>
                          <td className="px-6 py-4 whitespace-nowrap">
                            <div className="w-20 h-8">
                              <ResponsiveContainer width="100%" height="100%">
                                <LineChart data={historicalData.slice(-7)}>
                                  <Line
                                    type="monotone"
                                    dataKey="score"
                                    stroke={
                                      anchor.reliability_score >= 95
                                        ? "#10b981"
                                        : anchor.reliability_score >= 85
                                          ? "#f59e0b"
                                          : "#ef4444"
                                    }
                                    strokeWidth={2}
                                    dot={false}
                                  />
                                </LineChart>
                              </ResponsiveContainer>
                            </div>
                          </td>
                          <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                            <Link
                              href={`/anchors/${anchor.stellar_account}`}
                              className="text-blue-600 dark:text-blue-400 hover:text-blue-900 dark:hover:text-blue-300 inline-flex items-center gap-1"
                            >
                              View Details
                              <ExternalLink className="w-3 h-3" />
                            </Link>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>

              {/* Mobile Cards */}
              <div className="lg:hidden divide-y divide-gray-200 dark:divide-slate-700">
                {paginatedAnchors.map((anchor) => {
                  const successRate =
                    (anchor.successful_transactions /
                      anchor.total_transactions) *
                    100;
                  const historicalData = generateMockHistoricalData(
                    anchor.reliability_score,
                  );

                  return (
                    <div key={anchor.id} className="p-4">
                      <div className="flex items-start justify-between mb-3">
                        <div className="flex items-center">
                          <div className="w-10 h-10 bg-blue-100 dark:bg-blue-900 rounded-lg flex items-center justify-center mr-3">
                            <Anchor className="w-5 h-5 text-blue-600 dark:text-blue-300" />
                          </div>
                          <div>
                            <div className="text-sm font-medium text-gray-900 dark:text-white">
                              {anchor.name}
                            </div>
                            <div className="text-xs text-gray-500 dark:text-gray-400 font-mono">
                              {truncateAddress(anchor.stellar_account)}
                            </div>
                          </div>
                        </div>
                        <span
                          className={`inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium ${getHealthStatusColor(anchor.status)}`}
                        >
                          {getHealthStatusIcon(anchor.status)}
                          {anchor.status}
                        </span>
                      </div>

                      <div className="grid grid-cols-2 gap-4 mb-3">
                        <div>
                          <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                            Reliability
                          </div>
                          <div className="flex items-center">
                            <span className="text-sm font-medium text-gray-900 dark:text-white mr-2">
                              {anchor.reliability_score.toFixed(1)}%
                            </span>
                            <div className="flex-1 bg-gray-200 dark:bg-slate-600 rounded-full h-2">
                              <div
                                className={`h-2 rounded-full ${anchor.reliability_score >= 95
                                    ? "bg-green-500"
                                    : anchor.reliability_score >= 85
                                      ? "bg-yellow-500"
                                      : "bg-red-500"
                                  }`}
                                style={{
                                  width: `${anchor.reliability_score}%`,
                                }}
                              />
                            </div>
                          </div>
                        </div>
                        <div>
                          <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                            Success Rate
                          </div>
                          <div className="text-sm font-medium text-gray-900 dark:text-white">
                            {successRate.toFixed(1)}%
                          </div>
                        </div>
                        <div>
                          <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                            Assets
                          </div>
                          <div className="text-sm font-medium text-gray-900 dark:text-white">
                            {anchor.asset_coverage}
                          </div>
                        </div>
                        <div>
                          <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                            Transactions
                          </div>
                          <div className="text-sm font-medium text-gray-900 dark:text-white">
                            {formatNumber(anchor.total_transactions)}
                          </div>
                        </div>
                      </div>

                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <BarChart3 className="w-4 h-4 text-gray-400" />
                          <span className="text-xs text-gray-500 dark:text-gray-400">
                            30-day trend
                          </span>
                          <div className="w-16 h-6">
                            <ResponsiveContainer width="100%" height="100%">
                              <LineChart data={historicalData.slice(-7)}>
                                <Line
                                  type="monotone"
                                  dataKey="score"
                                  stroke={
                                    anchor.reliability_score >= 95
                                      ? "#10b981"
                                      : anchor.reliability_score >= 85
                                        ? "#f59e0b"
                                        : "#ef4444"
                                  }
                                  strokeWidth={2}
                                  dot={false}
                                />
                              </LineChart>
                            </ResponsiveContainer>
                          </div>
                        </div>
                        <Link
                          href={`/anchors/${anchor.stellar_account}`}
                          className="text-blue-600 dark:text-blue-400 hover:text-blue-900 dark:hover:text-blue-300 inline-flex items-center gap-1 text-sm"
                        >
                          Details
                          <ExternalLink className="w-3 h-3" />
                        </Link>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>

            <DataTablePagination
              totalItems={filteredAndSortedAnchors.length}
              pageSize={pageSize}
              currentPage={currentPage}
              onPageChange={onPageChange}
              onPageSizeChange={onPageSizeChange}
            />
          </div>
        )}

        {/* Summary Stats */}
        {!loading && filteredAndSortedAnchors.length > 0 && (
          <div className="mt-8 grid grid-cols-1 md:grid-cols-4 gap-4">
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">
                Total Anchors
              </div>
              <div className="text-2xl font-bold text-gray-900 dark:text-white">
                {filteredAndSortedAnchors.length}
              </div>
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">
                Avg Reliability
              </div>
              <div className="text-2xl font-bold text-gray-900 dark:text-white">
                {(
                  filteredAndSortedAnchors.reduce(
                    (sum, a) => sum + a.reliability_score,
                    0,
                  ) / filteredAndSortedAnchors.length
                ).toFixed(1)}
                %
              </div>
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">
                Total Transactions
              </div>
              <div className="text-2xl font-bold text-gray-900 dark:text-white">
                {formatNumber(
                  filteredAndSortedAnchors.reduce(
                    (sum, a) => sum + a.total_transactions,
                    0,
                  ),
                )}
              </div>
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">
                Healthy Anchors
              </div>
              <div className="text-2xl font-bold text-green-600 dark:text-green-400">
                {
                  filteredAndSortedAnchors.filter((a) => a.status === "Healthy")
                    .length
                }
              </div>
            </div>
          </div>
        )}
      </div>
    </MainLayout>
  );
}

export default function AnchorsPage() {
  return (
    <Suspense fallback={
      <MainLayout>
        <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto flex items-center justify-center h-64">
          <Loader className="w-8 h-8 animate-spin text-blue-500" />
        </div>
      </MainLayout>
    }>
      <AnchorsPageContent />
    </Suspense>
  );
}
