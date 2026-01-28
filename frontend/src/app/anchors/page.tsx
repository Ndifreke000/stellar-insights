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
        </div>

        {/* Anchor Table */}
        <div className="space-y-4">
          <AnchorTable anchors={paginatedAnchors} loading={loading} />

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