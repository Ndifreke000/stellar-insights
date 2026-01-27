"use client";

import React, { Suspense, useEffect, useState } from "react";
import {
  TrendingUp,
  Search,
  Filter,
  Droplets,
  CheckCircle2,
  AlertCircle,
  ArrowRight,
  X,
  Save,
  RotateCcw,
} from "lucide-react";
import Link from "next/link";
import {
  getCorridors,
  generateMockCorridorData,
  CorridorMetrics,
} from "@/lib/api";
import { MainLayout } from "@/components/layout";
import { SkeletonCorridorCard } from "@/components/ui/Skeleton";
import { usePagination } from "@/hooks/usePagination";
import { DataTablePagination } from "@/components/ui/DataTablePagination";

export default function CorridorsPage() {
  return (
    <Suspense>
      <CorridorsContent />
    </Suspense>
  );
}

function CorridorsContent() {
  const [corridors, setCorridors] = useState<CorridorMetrics[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState("");
  const [sortBy, setSortBy] = useState<
    "success_rate" | "health_score" | "liquidity"
  >("health_score");
  
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

  const {
    startIndex,
    endIndex,
  } = usePagination(0);

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
          console.log("API not available, using mock data");
          // Generate mock corridors (same as before)
          const mockCorridors: CorridorMetrics[] = [
            {
              ...generateMockCorridorData("USDC-PHP").corridor,
              id: "USDC-PHP",
              source_asset: "USDC",
              destination_asset: "PHP",
            },
            {
              ...generateMockCorridorData("USDC-JPY").corridor,
              id: "USDC-JPY",
              source_asset: "USDC",
              destination_asset: "JPY",
            },
            {
              ...generateMockCorridorData("USDC-INR").corridor,
              id: "USDC-INR",
              source_asset: "USDC",
              destination_asset: "INR",
              success_rate: 95.2,
              total_attempts: 2100,
              successful_payments: 2000,
              failed_payments: 100,
              average_latency_ms: 420,
              median_latency_ms: 320,
              p95_latency_ms: 1100,
              p99_latency_ms: 1800,
              liquidity_depth_usd: 8500000,
              liquidity_volume_24h_usd: 1200000,
              liquidity_trend: "increasing",
              health_score: 96,
            },
            {
              ...generateMockCorridorData("USDC-KES").corridor,
              id: "USDC-KES",
              source_asset: "USDC",
              destination_asset: "KES",
              success_rate: 81.3,
              total_attempts: 950,
              successful_payments: 772,
              failed_payments: 178,
              average_latency_ms: 650,
              median_latency_ms: 520,
              p95_latency_ms: 1800,
              p99_latency_ms: 2500,
              liquidity_depth_usd: 2800000,
              liquidity_volume_24h_usd: 320000,
              liquidity_trend: "decreasing",
              health_score: 72,
            },
            {
              ...generateMockCorridorData("USDC-EUR").corridor,
              id: "USDC-EUR",
              source_asset: "USDC",
              destination_asset: "EUR",
              success_rate: 97.8,
              total_attempts: 3200,
              successful_payments: 3130,
              failed_payments: 70,
              average_latency_ms: 380,
              median_latency_ms: 280,
              p95_latency_ms: 950,
              p99_latency_ms: 1500,
              liquidity_depth_usd: 12000000,
              liquidity_volume_24h_usd: 2500000,
              liquidity_trend: "increasing",
              health_score: 98,
            },
            {
              ...generateMockCorridorData("USDC-GBP").corridor,
              id: "USDC-GBP",
              source_asset: "USDC",
              destination_asset: "GBP",
              success_rate: 94.1,
              total_attempts: 2450,
              successful_payments: 2305,
              failed_payments: 145,
              average_latency_ms: 410,
              median_latency_ms: 310,
              p95_latency_ms: 1050,
              p99_latency_ms: 1700,
              liquidity_depth_usd: 9800000,
              liquidity_volume_24h_usd: 1800000,
              liquidity_trend: "stable",
              health_score: 91,
            },
          ];
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

  // Filter by search term (additional client-side filtering)
  const filteredCorridors = corridors
    .filter(
      (c) =>
        searchTerm === "" ||
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

  const paginatedCorridors = filteredCorridors.slice(startIndex, endIndex);

  const {
    currentPage: finalCurrentPage,
    pageSize: finalPageSize,
    onPageChange: finalOnPageChange,
    onPageSizeChange: finalOnPageSizeChange,
  } = usePagination(filteredCorridors.length);

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

  // Filter functions
  const clearAllFilters = () => {
    setSuccessRateRange([0, 100]);
    setVolumeRange([0, 10000000]);
    setAssetCodeFilter("");
    setTimePeriod("");
    setSearchTerm("");
  };

  const saveFilterPreset = () => {
    if (!presetName.trim()) return;
    const preset = {
      name: presetName,
      filters: {
        successRateRange,
        volumeRange,
        assetCodeFilter,
        timePeriod,
        searchTerm,
        sortBy,
      },
    };
    const updatedPresets = [...filterPresets, preset];
    setFilterPresets(updatedPresets);
    localStorage.setItem('corridorFilterPresets', JSON.stringify(updatedPresets));
    setPresetName("");
  };

  const loadFilterPreset = (preset: typeof filterPresets[0]) => {
    setSuccessRateRange(preset.filters.successRateRange);
    setVolumeRange(preset.filters.volumeRange);
    setAssetCodeFilter(preset.filters.assetCodeFilter);
    setTimePeriod(preset.filters.timePeriod);
    setSearchTerm(preset.filters.searchTerm);
    setSortBy(preset.filters.sortBy);
  };

  const deleteFilterPreset = (index: number) => {
    const updatedPresets = filterPresets.filter((_preset, i) => i !== index);
    setFilterPresets(updatedPresets);
    localStorage.setItem('corridorFilterPresets', JSON.stringify(updatedPresets));
  };

  // Load presets on mount
  useEffect(() => {
    const saved = localStorage.getItem('corridorFilterPresets');
    if (saved) {
      setFilterPresets(JSON.parse(saved));
    }
  }, []);

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
            <button
              onClick={() => setShowFilters(!showFilters)}
              className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg px-4 py-2 text-gray-900 dark:text-white hover:bg-gray-50 dark:hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              {showFilters ? 'Hide Filters' : 'Show Filters'}
            </button>
          </div>
        </div>

        {/* Advanced Filters */}
        {showFilters && (
          <div className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-6 mb-6">
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-6">
              {/* Success Rate Range */}
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  Success Rate: {successRateRange[0]}% - {successRateRange[1]}%
                </label>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={successRateRange[0]}
                  onChange={(e) => setSuccessRateRange([parseInt(e.target.value), successRateRange[1]])}
                  className="w-full"
                />
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={successRateRange[1]}
                  onChange={(e) => setSuccessRateRange([successRateRange[0], parseInt(e.target.value)])}
                  className="w-full mt-2"
                />
              </div>

              {/* Volume Range */}
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  Volume: ${(volumeRange[0] / 1000000).toFixed(1)}M - ${(volumeRange[1] / 1000000).toFixed(1)}M
                </label>
                <input
                  type="range"
                  min="0"
                  max="10000000"
                  step="100000"
                  value={volumeRange[0]}
                  onChange={(e) => setVolumeRange([parseInt(e.target.value), volumeRange[1]])}
                  className="w-full"
                />
                <input
                  type="range"
                  min="0"
                  max="10000000"
                  step="100000"
                  value={volumeRange[1]}
                  onChange={(e) => setVolumeRange([volumeRange[0], parseInt(e.target.value)])}
                  className="w-full mt-2"
                />
              </div>

              {/* Asset Code Filter */}
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  Asset Code
                </label>
                <input
                  type="text"
                  placeholder="e.g., USDC, EURC"
                  value={assetCodeFilter}
                  onChange={(e) => setAssetCodeFilter(e.target.value)}
                  className="w-full bg-white dark:bg-slate-700 border border-gray-200 dark:border-slate-600 rounded-lg px-3 py-2 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              {/* Time Period */}
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  Time Period
                </label>
                <select
                  value={timePeriod}
                  onChange={(e) => setTimePeriod(e.target.value as "7d" | "30d" | "90d" | "")}
                  className="w-full bg-white dark:bg-slate-700 border border-gray-200 dark:border-slate-600 rounded-lg px-3 py-2 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  <option value="">Daily</option>
                  <option value="7d">Last 7 days</option>
                  <option value="30d">Last 30 days</option>
                  <option value="90d">Last 90 days</option>
                </select>
              </div>
            </div>

            {/* Filter Actions */}
            <div className="flex flex-wrap items-center gap-4 pt-4 border-t border-gray-200 dark:border-slate-600">
              <button
                onClick={clearAllFilters}
                className="flex items-center gap-2 bg-gray-100 dark:bg-slate-700 text-gray-700 dark:text-gray-300 px-4 py-2 rounded-lg hover:bg-gray-200 dark:hover:bg-slate-600 transition-colors"
              >
                <RotateCcw className="w-4 h-4" />
                Clear All Filters
              </button>

              {/* Save Preset */}
              <div className="flex items-center gap-2">
                <input
                  type="text"
                  placeholder="Preset name"
                  value={presetName}
                  onChange={(e) => setPresetName(e.target.value)}
                  className="bg-white dark:bg-slate-700 border border-gray-200 dark:border-slate-600 rounded-lg px-3 py-2 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                />
                <button
                  onClick={saveFilterPreset}
                  disabled={!presetName.trim()}
                  className="flex items-center gap-2 bg-blue-500 text-white px-4 py-2 rounded-lg hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                >
                  <Save className="w-4 h-4" />
                  Save Preset
                </button>
              </div>

              {/* Load Presets */}
              {filterPresets.length > 0 && (
                <div className="flex items-center gap-2">
                  <span className="text-sm text-gray-600 dark:text-gray-400">Load:</span>
                  {filterPresets.map((preset, index) => (
                    <div key={index} className="flex items-center gap-1">
                      <button
                        onClick={() => loadFilterPreset(preset)}
                        className="text-blue-500 hover:text-blue-600 text-sm underline"
                      >
                        {preset.name}
                      </button>
                      <button
                        onClick={() => deleteFilterPreset(index)}
                        className="text-red-500 hover:text-red-600"
                      >
                        <X className="w-3 h-3" />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}

        {/* Content */}
        {loading ? (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
            <SkeletonCorridorCard />
            <SkeletonCorridorCard />
            <SkeletonCorridorCard />
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
        ) : (
          <div className="space-y-6">
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
              {paginatedCorridors.map((corridor) => (
                <Link
                  key={corridor.id}
                  href={`/corridors/${corridor.id}`}
                  className="group bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg p-6 hover:border-blue-500 hover:shadow-lg transition-all duration-200 transform hover:-translate-y-1 text-left cursor-pointer"
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
              ))}
            </div>

            <DataTablePagination
              totalItems={filteredCorridors.length}
              pageSize={finalPageSize}
              currentPage={finalCurrentPage}
              onPageChange={finalOnPageChange}
              onPageSizeChange={finalOnPageSizeChange}
            />
          </div>
        )}

        {/* Info Footer */}
        <div className="mt-8 p-4 bg-gray-50 dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg text-gray-600 dark:text-gray-400 text-sm">
          <p>
            Showing {filteredCorridors.length} of {corridors.length} corridors
          </p>
          <p className="mt-2 text-xs">
            Click any card to view detailed analytics
          </p>
        </div>
      </div>
    </MainLayout>
  );
}
