"use client";

import React, { useEffect, useState, useMemo, Suspense } from "react";
import {
  TrendingUp,
  Search,
  Filter,
  Grid3x3,
  List,
  Droplets,
  CheckCircle2,
  AlertCircle,
  ArrowRight,
  Check,
  BarChart3,
  Plus,
  X,
} from "lucide-react";
import Link from "next/link";
import {
  getCorridors,
  CorridorMetrics,
} from "@/lib/api";
import { mockCorridors } from "@/components/lib/mockCorridorData";
import { MainLayout } from "@/components/layout";
import { SkeletonCorridorCard } from "@/components/ui/Skeleton";
import { CorridorHeatmap } from "@/components/charts/CorridorHeatmap";
import { usePagination } from "@/hooks/usePagination";
import { DataTablePagination } from "@/components/ui/DataTablePagination";

function CorridorsPageContent() {
  const [corridors, setCorridors] = useState<CorridorMetrics[]>([]);
  const [viewMode, setViewMode] = useState<"grid" | "heatmap">("grid");
  const [loading, setLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState("");
  const [sortBy, setSortBy] = useState<
    "success_rate" | "health_score" | "liquidity"
  >("health_score");
  const [selectedCorridors, setSelectedCorridors] = useState<string[]>([]);

  // Filter state variables
  const [successRateRange, setSuccessRateRange] = useState<[number, number]>([0, 100]);
  const [volumeRange, setVolumeRange] = useState<[number, number]>([0, 10000000]);
  const [assetCodeFilter, setAssetCodeFilter] = useState("");
  const [timePeriod, setTimePeriod] = useState("7d");
  const [showFilters, setShowFilters] = useState(false);

  // Filter presets state
  const [filterPresets, setFilterPresets] = useState<Array<{
    name: string;
    filters: {
      successRateRange: [number, number];
      volumeRange: [number, number];
      assetCodeFilter: string;
      timePeriod: string;
      searchTerm: string;
      sortBy: "success_rate" | "health_score" | "liquidity";
    };
  }>>([]);
  const [presetName, setPresetName] = useState("");

  const filteredCorridors = useMemo(() => {
    return corridors
      .filter(
        (c) =>
          c.source_asset.toLowerCase().includes(searchTerm.toLowerCase()) ||
          c.destination_asset.toLowerCase().includes(searchTerm.toLowerCase()) ||
          c.id.toLowerCase().includes(searchTerm.toLowerCase()),
      )
      .sort((a, b) => {
        switch (sortBy) {
          case "success_rate":
            return b.success_rate - a.success_rate;
          case "liquidity":
            return b.liquidity_depth_usd - a.liquidity_depth_usd;
          case "health_score":
          default:
            return b.health_score - a.health_score;
        }
      });
  }, [corridors, searchTerm, sortBy]);

  const {
    currentPage,
    pageSize,
    onPageChange,
    onPageSizeChange,
    startIndex,
    endIndex,
  } = usePagination(filteredCorridors.length);

  useEffect(() => {
    async function fetchCorridors() {
      try {
        setLoading(true);
        try {
          const filters: Record<string, string | number> = {};
          if (successRateRange[0] > 0) filters.success_rate_min = successRateRange[0];
          if (successRateRange[1] < 100) filters.success_rate_max = successRateRange[1];
          if (volumeRange[0] > 0) filters.volume_min = volumeRange[0];
          if (volumeRange[1] < 10000000) filters.volume_max = volumeRange[1];
          if (assetCodeFilter) filters.asset_code = assetCodeFilter;
          if (timePeriod) filters.time_period = timePeriod;
          filters.sort_by = sortBy;

          const result = await getCorridors(filters);
          setCorridors(result);
        } catch {
          // Backend API not available - gracefully fall back to mock data
          // This is expected behavior when the backend server isn't running
          setCorridors(mockCorridors);
        }
      } catch (err) {
        console.error("Error fetching corridors:", err);
      } finally {
        setLoading(false);
      }
    }

    fetchCorridors();
  }, [successRateRange, volumeRange, assetCodeFilter, timePeriod, sortBy]);

  const paginatedCorridors = filteredCorridors.slice(startIndex, endIndex);

  const getHealthColor = (score: number) => {
    if (score >= 90)
      return "bg-green-50 dark:bg-green-900/20 border-green-500/50 text-green-600 dark:text-green-400";
    if (score >= 75)
      return "bg-yellow-50 dark:bg-yellow-900/20 border-yellow-500/50 text-yellow-600 dark:text-yellow-400";
    return "bg-red-50 dark:bg-red-900/20 border-red-500/50 text-red-600 dark:text-red-400";
  };

  const getHealthStatus = (
    score: number,
  ): { label: string; icon: string; color: string } => {
    if (score >= 90)
      return { label: "Robust", icon: "🟢", color: "text-green-500" };
    if (score >= 75)
      return { label: "Moderate", icon: "🟡", color: "text-yellow-500" };
    return { label: "Fragile", icon: "🔴", color: "text-red-500" };
  };

  const getSuccessStatusIcon = (rate: number) => {
    if (rate >= 90) return <CheckCircle2 className="w-5 h-5 text-green-500" />;
    if (rate >= 75) return <TrendingUp className="w-5 h-5 text-yellow-500" />;
    return <AlertCircle className="w-5 h-5 text-red-500" />;
  };

  const toggleCorridor = (id: string, e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setSelectedCorridors((prev) => {
      if (prev.includes(id)) {
        return prev.filter((item) => item !== id);
      }
      if (prev.length >= 3) {
        return [...prev.slice(1), id];
      }
      return [...prev, id];
    });
  };

  return (
    <MainLayout>
      <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto">
        {/* Page Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-2 flex items-center gap-2">
            <TrendingUp className="w-8 h-8 text-blue-500" />
            Payment Corridors
          </h1>
          <p className="text-gray-600 dark:text-gray-400">
            Explore payment routes and their performance metrics
          </p>
        </div>

        {/* Search and Filter */}
        <div className="flex flex-col sm:flex-row gap-4 mb-6">
          <div className="flex-1 relative">
            <Search className="absolute left-4 top-3 w-5 h-5 text-gray-400" />
            <input
              type="text"
              placeholder="Search corridors..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="w-full bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg pl-10 pr-4 py-2 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
          <div className="flex items-center gap-2">
            <Filter className="w-5 h-5 text-gray-400" />
            <select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as "success_rate" | "health_score" | "liquidity")}
              className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg px-4 py-2 text-gray-900 dark:text-white appearance-none cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="health_score">Sort by Health Score</option>
              <option value="success_rate">Sort by Success Rate</option>
              <option value="liquidity">Sort by Liquidity</option>
            </select>
          </div>
        </div>

        {/* View Mode Toggle */}
        <div className="flex items-center gap-2 bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-1 mb-6">
          <button
            onClick={() => setViewMode("grid")}
            className={`flex items-center gap-2 px-3 py-1.5 rounded transition-colors ${viewMode === "grid"
              ? "bg-blue-500 text-white"
              : "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-slate-700"
              }`}
          >
            <List className="w-4 h-4" />
            <span className="text-sm font-medium">Grid</span>
          </button>
          <button
            onClick={() => setViewMode("heatmap")}
            className={`flex items-center gap-2 px-3 py-1.5 rounded transition-colors ${viewMode === "heatmap"
              ? "bg-blue-500 text-white"
              : "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-slate-700"
              }`}
          >
            <Grid3x3 className="w-4 h-4" />
            <span className="text-sm font-medium">Heatmap</span>
          </button>
        </div>

        {/* Content */}
        {loading ? (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
            <SkeletonCorridorCard />
            <SkeletonCorridorCard />
            <SkeletonCorridorCard />
          </div>
        ) : filteredCorridors.length === 0 ? (
          <div className="text-center py-12">
            <AlertCircle className="w-12 h-12 text-gray-400 mx-auto mb-4" />
            <p className="text-gray-600 dark:text-gray-400">
              No corridors found
            </p>
          </div>
        ) : viewMode === "heatmap" ? (
          <div className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-6">
            <CorridorHeatmap corridors={filteredCorridors} />
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
            {filteredCorridors.map((corridor) => (
              <div key={corridor.id} className="relative group">

                <div className="absolute top-4 right-4 z-10">
                  <button
                    onClick={(e) => toggleCorridor(corridor.id, e)}
                    className={`w-6 h-6 rounded-md border flex items-center justify-center transition-all ${selectedCorridors.includes(corridor.id)
                      ? 'bg-blue-600 border-blue-600 text-white shadow-md'
                      : 'bg-white/80 dark:bg-slate-700/80 border-gray-300 dark:border-slate-600 hover:border-blue-400'
                      }`}
                    title={selectedCorridors.includes(corridor.id) ? "Deselect for comparison" : "Select for comparison"}
                  >
                    {selectedCorridors.includes(corridor.id) ? (
                      <Check className="w-4 h-4 stroke-[3]" />
                    ) : (
                      <Plus className="w-4 h-4 text-gray-400" />
                    )}
                  </button>
                </div>
                <Link
                  href={`/corridors/${corridor.id}`}
                  className={`block bg-white dark:bg-slate-800 border rounded-xl p-6 hover:shadow-xl transition-all duration-300 transform hover:-translate-y-1 text-left cursor-pointer h-full ${selectedCorridors.includes(corridor.id)
                    ? 'border-blue-500 ring-2 ring-blue-500/20 shadow-blue-500/10'
                    : 'border-gray-200 dark:border-slate-700'
                    }`}
                >
                  {/* Header */}
                  <div className="flex items-start justify-between mb-4">
                    <div className="flex-1 min-w-0">
                      <h2 className="text-xl font-bold text-gray-900 dark:text-white group-hover:text-blue-500 transition-colors truncate">
                        {corridor.source_asset} → {corridor.destination_asset}
                      </h2>
                      <p className="text-sm text-gray-500 dark:text-gray-400 mt-1 truncate">
                        {corridor.id}
                      </p>
                    </div>
                    <ArrowRight className="w-5 h-5 text-gray-400 group-hover:text-blue-500 transition-colors opacity-0 group-hover:opacity-100 transform translate-x-2 group-hover:translate-x-0 duration-200 shrink-0 ml-2" />
                  </div>

                  {/* Success Rate and Health Score */}
                  <div className="grid grid-cols-2 gap-3 mb-4">
                    <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-3">
                      <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
                        Success Rate
                      </p>
                      <div className="flex items-center gap-2">
                        {getSuccessStatusIcon(corridor.success_rate)}
                        <p className="text-lg font-bold text-green-600 dark:text-green-400">
                          {corridor.success_rate.toFixed(1)}%
                        </p>
                      </div>
                    </div>
                    <div
                      className={`rounded-lg p-3 border ${getHealthColor(
                        corridor.health_score,
                      )}`}
                    >
                      <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
                        Health
                      </p>
                      <p className="text-lg font-bold text-gray-900 dark:text-white">
                        {corridor.health_score.toFixed(0)}
                      </p>
                      <div className="flex items-center gap-1 mt-1">
                        <span className="text-xs">
                          {getHealthStatus(corridor.health_score).icon}
                        </span>
                        <span
                          className={`text-xs font-semibold ${getHealthStatus(corridor.health_score).color}`}
                        >
                          {getHealthStatus(corridor.health_score).label}
                        </span>
                      </div>
                    </div>
                  </div>

                  {/* Metrics */}
                  <div className="space-y-2 text-sm mb-4">
                    <div className="flex justify-between items-center">
                      <span className="text-gray-600 dark:text-gray-400">
                        Avg Latency
                      </span>
                      <span className="font-semibold text-blue-600 dark:text-blue-400">
                        {corridor.average_latency_ms.toFixed(0)}ms
                      </span>
                    </div>
                    <div className="flex justify-between items-center">
                      <span className="text-gray-600 dark:text-gray-400 flex items-center gap-1">
                        <Droplets className="w-4 h-4" />
                        Liquidity
                      </span>
                      <span className="font-semibold text-purple-600 dark:text-purple-400">
                        ${(corridor.liquidity_depth_usd / 1000000).toFixed(1)}M
                      </span>
                    </div>
                    <div className="flex justify-between items-center">
                      <span className="text-gray-600 dark:text-gray-400">
                        24h Volume
                      </span>
                      <span className="font-semibold text-amber-600 dark:text-amber-400">
                        $
                        {(corridor.liquidity_volume_24h_usd / 1000000).toFixed(2)}
                        M
                      </span>
                    </div>
                  </div>

                  {/* Payment Attempts */}
                  <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-3">
                    <div className="flex justify-between items-center text-xs text-gray-600 dark:text-gray-400">
                      <span>{corridor.successful_payments} successful</span>
                      <span>{corridor.failed_payments} failed</span>
                    </div>
                    <div className="w-full bg-gray-200 dark:bg-slate-600 rounded-full h-2 mt-2">
                      <div
                        className="bg-green-500 rounded-full h-full transition-all duration-300"
                        style={{
                          width: `${(corridor.successful_payments / corridor.total_attempts) * 100}%`,
                        }}
                      />
                    </div>
                  </div>
                </Link>
              </div>
            ))}
          </div>
        )}

        {/* Comparison Floating Bar */}
        {selectedCorridors.length > 0 && (
          <div className="fixed bottom-8 left-1/2 -translate-x-1/2 z-50 animate-in slide-in-from-bottom-10 duration-500">
            <div className="bg-slate-900 border border-slate-700 rounded-2xl shadow-2xl p-4 flex items-center gap-6 backdrop-blur-md bg-slate-900/90">
              <div className="flex items-center gap-2 px-2">
                <BarChart3 className="w-5 h-5 text-blue-400" />
                <span className="text-white font-bold">{selectedCorridors.length} selected</span>
              </div>

              <div className="flex gap-2">
                {selectedCorridors.map(id => (
                  <div key={id} className="bg-slate-800 text-slate-300 px-3 py-1 rounded-lg text-xs font-medium border border-slate-700 flex items-center gap-2">
                    {id}
                    <button onClick={(e) => toggleCorridor(id, e)} className="hover:text-red-400 transition-colors">
                      <X className="w-3 h-3" />
                    </button>
                  </div>
                ))}
              </div>

              <div className="h-8 w-px bg-slate-700 mx-2" />

              <div className="flex items-center gap-3">
                <button
                  onClick={() => setSelectedCorridors([])}
                  className="text-slate-400 hover:text-white text-sm font-medium transition-colors"
                >
                  Clear
                </button>
                <Link
                  href={`/corridors/compare?ids=${selectedCorridors.join(",")}`}
                  className="bg-blue-600 hover:bg-blue-500 text-white px-6 py-2 rounded-xl font-bold transition-all shadow-lg shadow-blue-500/20 flex items-center gap-2"
                >
                  Compare Now
                  <ArrowRight className="w-4 h-4" />
                </Link>
              </div>
            </div>
          </div>
        )}

        {/* Info Footer */}
        <div className="mt-8 p-4 bg-gray-50 dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg text-gray-600 dark:text-gray-400 text-sm">
          <p>
            Showing {filteredCorridors.length} of {corridors.length} corridors
          </p>
          <div className="mt-2 flex flex-col gap-1 text-xs">
            <p>• Click any card to view detailed analytics</p>
            <p>• Select up to 3 corridors to compare performance metrics side-by-side</p>
          </div>
        </div>
      </div>
    </MainLayout>
  );
}

export default function CorridorsPage() {
  return (
    <Suspense fallback={
      <MainLayout>
        <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto flex items-center justify-center h-64">
          <div className="w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full animate-spin" />
        </div>
      </MainLayout>
    }>
      <CorridorsPageContent />
    </Suspense>
  );
}
