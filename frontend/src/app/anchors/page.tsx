"use client";

import React, { useState, useEffect, useMemo } from "react";
import { Search, Anchor as AnchorIcon } from "lucide-react";
import { MainLayout } from "@/components/layout";
import { AnchorMetrics, fetchAnchors} from "@/lib/api";
import AnchorTable from "@/components/tables/AnchorsTables";
import { usePagination } from "@/hooks/usePagination";
import { DataTablePagination } from "@/components/ui/DataTablePagination";

const AnchorsPage = () => {
  const [anchors, setAnchors] = useState<AnchorMetrics[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState("");

  // Fetch anchors from the backend
  useEffect(() => {
    const loadAnchors = async () => {
      try {
        setLoading(true);
        setError(null);
        
        // Fetch data from the backend API
        const response = await fetchAnchors({ limit: 100, offset: 0 });
        setAnchors(response.anchors);
      } catch (err) {
        console.error("Failed to fetch anchors:", err);
        setError(err instanceof Error ? err.message : "Failed to load anchors");
      } finally {
        setLoading(false);
      }
    };

    loadAnchors();
  }, []);

  // Filter anchors based on search
  const filteredAnchors = useMemo(() => {
    return anchors.filter(
      (anchor) =>
        anchor.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        anchor.stellar_account.toLowerCase().includes(searchTerm.toLowerCase())
    );
  }, [anchors, searchTerm]);

  // Pagination
  const {
    currentPage,
    pageSize,
    onPageChange,
    onPageSizeChange,
    startIndex,
    endIndex,
  } = usePagination(filteredAnchors.length);

  const paginatedAnchors = filteredAnchors.slice(startIndex, endIndex);

  // Helper functions for stats
  const formatNumber = (num: number) => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  };

  return (
    <MainLayout>
      <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto">
        {/* Page Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-2 flex items-center gap-2">
            <AnchorIcon className="w-8 h-8 text-blue-500" />
            Anchor Analytics
          </h1>
          <p className="text-gray-600 dark:text-gray-400">
            Monitor anchor reliability, asset coverage, and transaction success rates
          </p>
        </div>

        {/* Error Message */}
        {error && (
          <div className="mb-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
            <div className="flex items-center gap-2">
              <div className="text-red-600 dark:text-red-400 font-medium">
                Error loading anchors
              </div>
            </div>
            <div className="text-sm text-red-600 dark:text-red-400 mt-1">
              {error}
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
              onClick={() => window.location.reload()}
              className="mt-2 text-sm text-red-600 dark:text-red-400 underline hover:no-underline"
            >
              Retry
            </button>
          </div>
        )}

        {/* Search Bar */}
        <div className="mb-6">
          <div className="relative">
            <Search className="absolute left-4 top-3 w-5 h-5 text-gray-400" />
            <input
              type="text"
              placeholder="Search anchors by name or address..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="w-full pl-12 pr-4 py-3 border border-gray-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 transition-shadow"
              disabled={loading}
            />
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

          {/* Pagination */}
          {!loading && !error && filteredAnchors.length > 0 && (
            <DataTablePagination
              totalItems={filteredAnchors.length}
              pageSize={pageSize}
              currentPage={currentPage}
              onPageChange={onPageChange}
              onPageSizeChange={onPageSizeChange}
            />
          )}
        </div>

        {/* Summary Stats */}
        {!loading && !error && filteredAnchors.length > 0 && (
          <div className="mt-8 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">
                Total Anchors
              </div>
              <div className="text-2xl font-bold text-gray-900 dark:text-white">
                {filteredAnchors.length}
              </div>
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">
                Avg Reliability
              </div>
              <div className="text-2xl font-bold text-gray-900 dark:text-white">
                {filteredAnchors.length > 0
                  ? (
                      filteredAnchors.reduce((sum, a) => sum + a.reliability_score, 0) /
                      filteredAnchors.length
                    ).toFixed(1)
                  : "0.0"}
                %
              </div>
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">
                Total Transactions
              </div>
              <div className="text-2xl font-bold text-gray-900 dark:text-white">
                {formatNumber(
                  filteredAnchors.reduce((sum, a) => sum + a.total_transactions, 0)
                )}
              </div>
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">
                Healthy Anchors
              </div>
              <div className="text-2xl font-bold text-green-600 dark:text-green-400">
                {filteredAnchors.filter((a) => a.status.toLowerCase() === "green" || a.status === "Healthy").length}
              </div>
            </div>
          </div>
        )}

        {/* Empty State (when no error but also no data) */}
        {!loading && !error && filteredAnchors.length === 0 && anchors.length > 0 && (
          <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-12 text-center">
            <Search className="w-12 h-12 text-gray-400 mx-auto mb-4" />
            <p className="text-gray-600 dark:text-gray-400">
              No anchors found matching "{searchTerm}"
            </p>
          </div>
        )}
      </div>
    </MainLayout>
  );
};

export default AnchorsPage;