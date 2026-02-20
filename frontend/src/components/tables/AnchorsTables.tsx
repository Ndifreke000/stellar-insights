"use client";

import React, { useState, useMemo } from "react";
import { useRouter } from "next/navigation";
import {
  Anchor,
  CheckCircle,
  AlertCircle,
  Activity,
  ExternalLink,
  BarChart3,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
} from "lucide-react";
import { AnchorMetrics } from "@/lib/api";
import { formatAddressShort } from "@/lib/address";
import { LineChart, Line, ResponsiveContainer } from "recharts";

interface AnchorTableProps {
  anchors: AnchorMetrics[];
  loading?: boolean;
}

type SortField = "name" | "reliability_score" | "failure_rate" | "total_transactions";
type SortOrder = "asc" | "desc";

// Map backend status to display status
const mapBackendStatus = (backendStatus: string): string => {
  const status = backendStatus.toLowerCase();
  switch (status) {
    case "green":
      return "Healthy";
    case "yellow":
      return "Warning";
    case "red":
      return "Critical";
    default:
      return backendStatus; // Return as-is if already in display format
  }
};

// Generate mock historical data for mini charts
const generateMockHistoricalData = (baseScore: number) => {
  const data = [];
  const now = new Date();

  for (let i = 6; i >= 0; i--) {
    const date = new Date(now.getTime() - i * 24 * 60 * 60 * 1000);
    const variation = ((date.getTime() / 1000) % 20) - 10;
    data.push({
      date: date.toISOString().split("T")[0],
      score: Math.max(0, Math.min(100, baseScore + variation)),
    });
  }

  return data;
};

const AnchorTable: React.FC<AnchorTableProps> = ({ anchors, loading = false }) => {
  const router = useRouter();
  const [sortField, setSortField] = useState<SortField>("reliability_score");
  const [sortOrder, setSortOrder] = useState<SortOrder>("desc");

  // Handle sorting
  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortOrder(sortOrder === "asc" ? "desc" : "asc");
    } else {
      setSortField(field);
      setSortOrder("desc");
    }
  };

  // Sort anchors
  const sortedAnchors = useMemo(() => {
    return [...anchors].sort((a, b) => {
      let aValue: number | string;
      let bValue: number | string;

      switch (sortField) {
        case "name":
          aValue = a.name.toLowerCase();
          bValue = b.name.toLowerCase();
          break;
        case "reliability_score":
          aValue = a.reliability_score;
          bValue = b.reliability_score;
          break;
        case "failure_rate":
          aValue = a.failure_rate;
          bValue = b.failure_rate;
          break;
        case "total_transactions":
          aValue = a.total_transactions;
          bValue = b.total_transactions;
          break;
        default:
          aValue = a.reliability_score;
          bValue = b.reliability_score;
      }

      if (typeof aValue === "string" && typeof bValue === "string") {
        return sortOrder === "asc"
          ? aValue.localeCompare(bValue)
          : bValue.localeCompare(aValue);
      }

      return sortOrder === "asc"
        ? (aValue as number) - (bValue as number)
        : (bValue as number) - (aValue as number);
    });
  }, [anchors, sortField, sortOrder]);

  // Handle row click
  const handleRowClick = (anchorId: string) => {
    router.push(`/anchors/${anchorId}`);
  };

  // Helper functions
  const getHealthStatusColor = (status: string) => {
    const displayStatus = mapBackendStatus(status);
    switch (displayStatus.toLowerCase()) {
      case "healthy":
        return "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300";
      case "warning":
        return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300";
      case "critical":
        return "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300";
      default:
        return "bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-300";
    }
  };

  const getHealthStatusIcon = (status: string) => {
    const displayStatus = mapBackendStatus(status);
    switch (displayStatus.toLowerCase()) {
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

  const truncateAddress = (address: string) =>
    formatAddressShort(address, 8, 8);

  const getSortIcon = (field: SortField) => {
    if (sortField !== field) {
      return <ArrowUpDown className="w-4 h-4 text-gray-400" />;
    }
    return sortOrder === "asc" ? (
      <ArrowUp className="w-4 h-4 text-blue-500" />
    ) : (
      <ArrowDown className="w-4 h-4 text-blue-500" />
    );
  };

  if (loading) {
    return (
      <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 overflow-hidden">
        <div className="animate-pulse p-6 space-y-4">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="h-16 bg-gray-200 dark:bg-slate-700 rounded"></div>
          ))}
        </div>
      </div>
    );
  }

  if (anchors.length === 0) {
    return (
      <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-12 text-center">
        <Anchor className="w-12 h-12 text-gray-400 mx-auto mb-4" />
        <p className="text-gray-600 dark:text-gray-400">No anchors available</p>
      </div>
    );
  }

  return (
    <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 overflow-hidden">
      {/* Desktop Table */}
      <div className="hidden lg:block overflow-x-auto">
        <table className="w-full">
          <thead className="bg-gray-50 dark:bg-slate-700/50">
            <tr>
              <th
                onClick={() => handleSort("name")}
                className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-slate-700 transition-colors"
              >
                <div className="flex items-center gap-2">
                  Anchor / Address
                  {getSortIcon("name")}
                </div>
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                Status
              </th>
              <th
                onClick={() => handleSort("reliability_score")}
                className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-slate-700 transition-colors"
              >
                <div className="flex items-center gap-2">
                  Reliability Score
                  {getSortIcon("reliability_score")}
                </div>
              </th>
              <th
                onClick={() => handleSort("failure_rate")}
                className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-slate-700 transition-colors"
              >
                <div className="flex items-center gap-2">
                  Failure Rate
                  {getSortIcon("failure_rate")}
                </div>
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                Success Rate
              </th>
              <th
                onClick={() => handleSort("total_transactions")}
                className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-slate-700 transition-colors"
              >
                <div className="flex items-center gap-2">
                  Transactions
                  {getSortIcon("total_transactions")}
                </div>
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                7-Day Trend
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-200 dark:divide-slate-700">
            {sortedAnchors.map((anchor) => {
              const successRate = anchor.total_transactions > 0 
                ? (anchor.successful_transactions / anchor.total_transactions) * 100 
                : 0;
              const historicalData = generateMockHistoricalData(anchor.reliability_score);
              const displayStatus = mapBackendStatus(anchor.status);

              return (
                <tr
                  key={anchor.id}
                  onClick={() => handleRowClick(anchor.stellar_account)}
                  className="hover:bg-gray-50 dark:hover:bg-slate-700/50 cursor-pointer transition-colors"
                >
                  <td className="px-6 py-4">
                    <div className="flex items-center">
                      <div className="w-10 h-10 bg-blue-100 dark:bg-blue-900/30 rounded-lg flex items-center justify-center mr-3 flex-shrink-0">
                        <Anchor className="w-5 h-5 text-blue-600 dark:text-blue-400" />
                      </div>
                      <div className="min-w-0">
                        <div className="text-sm font-medium text-gray-900 dark:text-white truncate">
                          {anchor.name}
                        </div>
                        <div className="text-xs text-gray-500 dark:text-gray-400 font-mono truncate">
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
                      {displayStatus}
                    </span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center gap-3">
                      <div className="text-sm font-semibold text-gray-900 dark:text-white">
                        {anchor.reliability_score.toFixed(1)}%
                      </div>
                      <div className="w-20 bg-gray-200 dark:bg-slate-600 rounded-full h-2">
                        <div
                          className={`h-2 rounded-full transition-all ${
                            anchor.reliability_score >= 95
                              ? "bg-green-500"
                              : anchor.reliability_score >= 85
                              ? "bg-yellow-500"
                              : "bg-red-500"
                          }`}
                          style={{ width: `${Math.min(anchor.reliability_score, 100)}%` }}
                        />
                      </div>
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="text-sm font-medium text-red-600 dark:text-red-400">
                      {anchor.failure_rate.toFixed(1)}%
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
                      {formatNumber(anchor.total_transactions)}
                    </div>
                    <div className="text-xs text-red-500 dark:text-red-400">
                      {formatNumber(anchor.failed_transactions)} failed
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="w-24 h-10">
                      <ResponsiveContainer width="100%" height="100%">
                        <LineChart data={historicalData}>
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
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {/* Mobile/Tablet Cards */}
      <div className="lg:hidden divide-y divide-gray-200 dark:divide-slate-700">
        {sortedAnchors.map((anchor) => {
          const successRate = anchor.total_transactions > 0 
            ? (anchor.successful_transactions / anchor.total_transactions) * 100 
            : 0;
          const historicalData = generateMockHistoricalData(anchor.reliability_score);
          const displayStatus = mapBackendStatus(anchor.status);

          return (
            <div
              key={anchor.id}
              onClick={() => handleRowClick(anchor.stellar_account)}
              className="p-4 hover:bg-gray-50 dark:hover:bg-slate-700/50 cursor-pointer transition-colors"
            >
              {/* Header */}
              <div className="flex items-start justify-between mb-3">
                <div className="flex items-center flex-1 min-w-0 mr-3">
                  <div className="w-10 h-10 bg-blue-100 dark:bg-blue-900/30 rounded-lg flex items-center justify-center mr-3 flex-shrink-0">
                    <Anchor className="w-5 h-5 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div className="min-w-0">
                    <div className="text-sm font-medium text-gray-900 dark:text-white truncate">
                      {anchor.name}
                    </div>
                    <div className="text-xs text-gray-500 dark:text-gray-400 font-mono truncate">
                      {truncateAddress(anchor.stellar_account)}
                    </div>
                  </div>
                </div>
                <span
                  className={`inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium ${getHealthStatusColor(anchor.status)} flex-shrink-0`}
                >
                  {getHealthStatusIcon(anchor.status)}
                  {displayStatus}
                </span>
              </div>

              {/* Metrics Grid */}
              <div className="grid grid-cols-2 gap-3 mb-3">
                <div>
                  <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                    Reliability Score
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-semibold text-gray-900 dark:text-white">
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
                        style={{ width: `${Math.min(anchor.reliability_score, 100)}%` }}
                      />
                    </div>
                  </div>
                </div>
                <div>
                  <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                    Failure Rate
                  </div>
                  <div className="text-sm font-semibold text-red-600 dark:text-red-400">
                    {anchor.failure_rate.toFixed(1)}%
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
                    Total Transactions
                  </div>
                  <div className="text-sm font-medium text-gray-900 dark:text-white">
                    {formatNumber(anchor.total_transactions)}
                  </div>
                </div>
              </div>

              {/* Trend */}
              <div className="flex items-center justify-between pt-3 border-t border-gray-200 dark:border-slate-700">
                <div className="flex items-center gap-2">
                  <BarChart3 className="w-4 h-4 text-gray-400" />
                  <span className="text-xs text-gray-500 dark:text-gray-400">7-day trend</span>
                  <div className="w-20 h-6">
                    <ResponsiveContainer width="100%" height="100%">
                      <LineChart data={historicalData}>
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
                <ExternalLink className="w-4 h-4 text-blue-600 dark:text-blue-400" />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default AnchorTable;