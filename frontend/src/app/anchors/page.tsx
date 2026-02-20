"use client";

import { useState, useEffect, useMemo, Suspense } from "react";
import { Search, Anchor as AnchorIcon, ExternalLink, BarChart3, ChevronUp, ChevronDown, ChevronsUpDown, CheckCircle, AlertCircle, Activity, TrendingUp, TrendingDown } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { ResponsiveContainer, LineChart, Line } from "recharts";
import { MainLayout } from "@/components/layout";
import { AnchorMetrics, fetchAnchors } from "@/lib/api";
import { usePagination } from "@/hooks/usePagination";
import { DataTablePagination } from "@/components/ui/DataTablePagination";
import { formatAddressShort } from "@/lib/address";

const truncateAddress = (address: string) =>
  formatAddressShort(address, 6, 4);

const getHealthStatusColor = (status: string) => {
  const normalizedStatus = status.toLowerCase();
  if (normalizedStatus === "green" || normalizedStatus === "healthy") {
    return "bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400";
  } else if (normalizedStatus === "yellow" || normalizedStatus === "warning") {
    return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-400";
  } else {
    return "bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400";
  }
};

const getHealthStatusIcon = (status: string) => {
  const normalizedStatus = status.toLowerCase();
  if (normalizedStatus === "green" || normalizedStatus === "healthy") {
    return "●";
  } else if (normalizedStatus === "yellow" || normalizedStatus === "warning") {
    return "●";
  } else {
    return "●";
  }
};

const generateMockHistoricalData = (currentScore: number) => {
  const data = [];
  for (let i = 30; i >= 0; i--) {
    const variation = (Math.random() - 0.5) * 10; // ±5 point variation
    const score = Math.max(0, Math.min(100, currentScore + variation));
    data.push({
      date: new Date(Date.now() - i * 24 * 60 * 60 * 1000).toISOString().split('T')[0],
      score: score
    });
  }
  return data;
};

// Sort handler function
const handleSort = (column: "reliability" | "transactions" | "failure_rate", currentSortBy: string, currentDirection: "asc" | "desc", setSortBy: (sort: "reliability" | "transactions" | "failure_rate") => void, setSortDirection: (dir: "asc" | "desc") => void) => {
  if (currentSortBy === column) {
    // Toggle direction if same column
    setSortDirection(currentDirection === "asc" ? "desc" : "asc");
  } else {
    // New column, default to desc for most metrics
    setSortBy(column);
    setSortDirection(column === "failure_rate" ? "asc" : "desc");
  }
};

// Sort indicator component
const SortIndicator = ({ column, currentSort, direction }: { column: string, currentSort: string, direction: "asc" | "desc" }) => {
  if (currentSort !== column) {
    return <ChevronsUpDown className="w-4 h-4 text-gray-400" />;
  }
  return direction === "asc" ? 
    <ChevronUp className="w-4 h-4 text-blue-500" /> : 
    <ChevronDown className="w-4 h-4 text-blue-500" />;
};

const AnchorsPageContent = () => {
  const router = useRouter();
  const [anchors, setAnchors] = useState<AnchorMetrics[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState("");
  const [sortBy, setSortBy] = useState<"reliability" | "transactions" | "failure_rate">("reliability");
  const [sortOrder, setSortOrder] = useState<"asc" | "desc">("desc");

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

  // Sort and paginate anchors
  const sortedAndFilteredAnchors = useMemo(() => {
    return [...filteredAnchors].sort((a, b) => {
      let comparison = 0;
      switch (sortBy) {
        case "reliability":
          comparison = b.reliability_score - a.reliability_score;
          break;
        case "transactions":
          comparison = b.total_transactions - a.total_transactions;
          break;
        case "failure_rate":
          comparison = a.failure_rate - b.failure_rate;
          break;
        default:
          return 0;
      }
      return sortOrder === "asc" ? -comparison : comparison;
    });
  }, [filteredAnchors, sortBy, sortOrder]);

  // Pagination
  const {
    currentPage,
    pageSize,
    onPageChange,
    onPageSizeChange,
    startIndex,
    endIndex,
  } = usePagination(sortedAndFilteredAnchors.length);

  const paginatedAnchors = useMemo(() => {
    return sortedAndFilteredAnchors.slice(startIndex, endIndex);
  }, [sortedAndFilteredAnchors, startIndex, endIndex]);

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
        )}

        <div className="mb-6 flex flex-col sm:flex-row gap-4 items-start sm:items-center justify-between">
          <div className="flex-1 relative w-full sm:max-w-md">
            <Search className="absolute left-3 top-2.5 w-5 h-5 text-gray-400" />
            <input
              type="text"
              placeholder="Search anchors by name or account..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="w-full pl-10 pr-4 py-2 border border-gray-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
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
        {!loading && !error && sortedAndFilteredAnchors.length > 0 && (
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-2 mb-4">
            💡 Click on any row to view anchor details • Click column headers to sort
          </p>
        )}
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
                      <th 
                        className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-slate-600 select-none"
                        onClick={() => handleSort("reliability", sortBy, sortOrder, setSortBy, setSortOrder)}
                      >
                        <div className="flex items-center gap-1">
                          Reliability Score
                          <SortIndicator column="reliability" currentSort={sortBy} direction={sortOrder} />
                        </div>
                      </th>
                      <th 
                        className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-slate-600 select-none"
                        onClick={() => handleSort("failure_rate", sortBy, sortOrder, setSortBy, setSortOrder)}
                      >
                        <div className="flex items-center gap-1">
                          Success Rate
                          <SortIndicator column="failure_rate" currentSort={sortBy} direction={sortOrder} />
                        </div>
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                        Asset Coverage
                      </th>
                      <th 
                        className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-slate-600 select-none"
                        onClick={() => handleSort("transactions", sortBy, sortOrder, setSortBy, setSortOrder)}
                      >
                        <div className="flex items-center gap-1">
                          Total Transactions
                          <SortIndicator column="transactions" currentSort={sortBy} direction={sortOrder} />
                        </div>
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
                          className="hover:bg-gray-50 dark:hover:bg-slate-700 cursor-pointer transition-colors"
                          onClick={() => router.push(`/anchors/${anchor.stellar_account}`)}
                        >
                          <td className="px-6 py-4 whitespace-nowrap">
                            <div className="flex items-center">
                              <div className="w-10 h-10 bg-blue-100 dark:bg-blue-900 rounded-lg flex items-center justify-center mr-3">
                                <AnchorIcon className="w-5 h-5 text-blue-600 dark:text-blue-300" />
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
                                  className={`h-2 rounded-full ${
                                    anchor.reliability_score >= 95
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
                              onClick={(e) => e.stopPropagation()}
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
                    <div 
                      key={anchor.id} 
                      className="p-4 cursor-pointer hover:bg-gray-50 dark:hover:bg-slate-700 transition-colors"
                      onClick={() => router.push(`/anchors/${anchor.stellar_account}`)}
                    >
                      <div className="flex items-start justify-between mb-3">
                        <div className="flex items-center">
                          <div className="w-10 h-10 bg-blue-100 dark:bg-blue-900 rounded-lg flex items-center justify-center mr-3">
                            <AnchorIcon className="w-5 h-5 text-blue-600 dark:text-blue-300" />
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
                                className={`h-2 rounded-full ${
                                  anchor.reliability_score >= 95
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
                          onClick={(e) => e.stopPropagation()}
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
          {!loading && !error && sortedAndFilteredAnchors.length > 0 && (
            <DataTablePagination
              totalItems={sortedAndFilteredAnchors.length}
              pageSize={pageSize}
              currentPage={currentPage}
              onPageChange={onPageChange}
              onPageSizeChange={onPageSizeChange}
            />
          )}
        </div>

        {/* Summary Stats */}
        {!loading && !error && sortedAndFilteredAnchors.length > 0 && (
          <div className="mt-8 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">
                Total Anchors
              </div>
              <div className="text-2xl font-bold text-gray-900 dark:text-white">
                {sortedAndFilteredAnchors.length}
              </div>
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">
                Avg Reliability
              </div>
              <div className="text-2xl font-bold text-gray-900 dark:text-white">
                {sortedAndFilteredAnchors.length > 0
                  ? (
                      sortedAndFilteredAnchors.reduce((sum, a) => sum + a.reliability_score, 0) /
                      sortedAndFilteredAnchors.length
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
                  sortedAndFilteredAnchors.reduce((sum, a) => sum + a.total_transactions, 0)
                )}
              </div>
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">
                Healthy Anchors
              </div>
              <div className="text-2xl font-bold text-green-600 dark:text-green-400">
                {sortedAndFilteredAnchors.filter((a) => a.status.toLowerCase() === "green" || a.status === "Healthy").length}
              </div>
            </div>
          </div>
        )}

        {/* Empty State (when no error but also no data) */}
        {!loading && !error && sortedAndFilteredAnchors.length === 0 && anchors.length > 0 && (
          <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-12 text-center">
            <Search className="w-12 h-12 text-gray-400 mx-auto mb-4" />
            <p className="text-gray-600 dark:text-gray-400">
              No anchors found matching &quot;{searchTerm}&quot;
            </p>
          </div>
        )}
      </div>
    </MainLayout>
  );
};

const AnchorsPage = () => {
  return (
    <Suspense fallback={
      <MainLayout>
        <div className="flex items-center justify-center min-h-screen">
          <div className="text-center">
            <AnchorIcon className="w-12 h-12 text-gray-400 mx-auto mb-4 animate-pulse" />
            <p className="text-gray-600 dark:text-gray-400">Loading anchors...</p>
          </div>
        </div>
      </MainLayout>
    }>
      <AnchorsPageContent />
    </Suspense>
  );
};

export default AnchorsPage;