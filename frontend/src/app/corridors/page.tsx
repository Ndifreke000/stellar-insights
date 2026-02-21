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
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import Link from "next/link";
import { getCorridors, CorridorMetrics } from "@/lib/api";
import { mockCorridors } from "@/components/lib//mockCorridorData";
import { MainLayout } from "@/components/layout";
import { SkeletonCorridorCard } from "@/components/ui/Skeleton";
import { CorridorHeatmap } from "@/components/charts/CorridorHeatmap";
import { DataTablePagination } from "@/components/ui/DataTablePagination";
import { usePagination } from "@/hooks/usePagination";
import {
  useUserPreferences,
  type CorridorsTimePeriod,
} from "@/contexts/UserPreferencesContext";

function CorridorsPageContent() {
  const { prefs, setPrefs } = useUserPreferences();

  const [corridors, setCorridors] = useState<CorridorMetrics[]>([]);
  // Persisted preferences
  const viewMode = prefs.corridorsViewMode;
  const setViewMode = (v: typeof viewMode) =>
    setPrefs({ corridorsViewMode: v });
  const sortBy = prefs.corridorsSortBy;
  const setSortBy = (v: typeof sortBy) => setPrefs({ corridorsSortBy: v });
  const timePeriod = prefs.corridorsTimePeriod;
  const setTimePeriod = (v: typeof timePeriod) =>
    setPrefs({ corridorsTimePeriod: v });

  const [loading, setLoading] = useState(true);
  // Volatile filters — intentionally session-only
  const [searchTerm, setSearchTerm] = useState("");
  // Filter state variables (volatile — session only)
  const [successRateRange, setSuccessRateRange] = useState<[number, number]>([
    0, 100,
  ]);
  const [volumeRange, setVolumeRange] = useState<[number, number]>([
    0, 10000000,
  ]);
  const [assetCodeFilter, setAssetCodeFilter] = useState("");
  const [showFilters, setShowFilters] = useState(false);

  // Filter presets state
  const [filterPresets, setFilterPresets] = useState<
    Array<{
      name: string;
      filters: {
        successRateRange: [number, number];
        volumeRange: [number, number];
        assetCodeFilter: string;
        timePeriod: string;
        searchTerm: string;
        sortBy: "success_rate" | "health_score" | "liquidity";
      };
    }>
  >([]);
  const [presetName, setPresetName] = useState("");

  const filteredCorridors = useMemo(() => {
    return corridors
      .filter(
        (c) =>
          c.source_asset.toLowerCase().includes(searchTerm.toLowerCase()) ||
          c.destination_asset
            .toLowerCase()
            .includes(searchTerm.toLowerCase()) ||
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
          if (successRateRange[0] > 0)
            filters.success_rate_min = successRateRange[0];
          if (successRateRange[1] < 100)
            filters.success_rate_max = successRateRange[1];
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
  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
      {/* Page Header */}
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
            Network Routing // 02
          </div>
          <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
            <TrendingUp className="w-8 h-8 text-accent" />
            Payment Corridors
          </h2>
        </div>
        <div className="flex items-center gap-3">
          <Badge
            variant="outline"
            className="text-[10px] font-mono border-accent/30 text-accent px-3 py-1 bg-accent/5"
          >
            {filteredCorridors.length} ACTIVE_ROUTES
          </Badge>
        </div>
      </div>

      {/* Search and Filter */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-4">
        <div className="lg:col-span-8 relative group">
          <Search className="absolute left-4 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground group-focus-within:text-accent transition-colors" />
          <input
            type="text"
            placeholder="Search Intelligence Database..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full bg-slate-900/50 border border-border/50 rounded-xl pl-11 pr-4 py-3 text-sm font-mono tracking-tight focus:outline-none focus:ring-2 focus:ring-accent/50 group-hover:border-accent/30 transition-all"
          />
        </div>
        <div className="lg:col-span-4 flex gap-2">
          <select
            value={timePeriod}
            onChange={(e) =>
              setTimePeriod(e.target.value as CorridorsTimePeriod)
            }
            className="flex-1 bg-slate-900/50 border border-border/50 rounded-xl px-4 py-3 text-[10px] font-bold uppercase tracking-widest focus:outline-none focus:ring-2 focus:ring-accent/50 appearance-none cursor-pointer"
          >
            <option value="7d">Time: 7 Days</option>
            <option value="30d">Time: 30 Days</option>
            <option value="90d">Time: 90 Days</option>
            <option value="">Time: All</option>
          </select>
          <select
            value={sortBy}
            onChange={(e) =>
              setSortBy(
                e.target.value as "success_rate" | "health_score" | "liquidity",
              )
            }
            className="flex-1 bg-slate-900/50 border border-border/50 rounded-xl px-4 py-3 text-[10px] font-bold uppercase tracking-widest focus:outline-none focus:ring-2 focus:ring-accent/50 appearance-none cursor-pointer"
          >
            <option value="health_score">Sort: Health</option>
            <option value="success_rate">Sort: Success</option>
            <option value="liquidity">Sort: Liquidity</option>
          </select>
          <button
            onClick={() => setShowFilters(!showFilters)}
            className={`p-3 border border-border/50 rounded-xl transition-all ${showFilters ? "bg-accent text-white border-accent" : "bg-slate-900/50 text-muted-foreground hover:border-accent/50"}`}
          >
            <Filter className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* View Mode Toggle */}
      <div className="flex items-center gap-1 p-1 bg-slate-950/50 border border-border/20 rounded-xl w-fit">
        <button
          onClick={() => setViewMode("grid")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-[10px] font-bold uppercase tracking-widest transition-all ${
            viewMode === "grid"
              ? "bg-accent text-white glow-accent"
              : "text-muted-foreground hover:text-foreground"
          }`}
        >
          <List className="w-3 h-3" />
          Grid
        </button>
        <button
          onClick={() => setViewMode("heatmap")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-[10px] font-bold uppercase tracking-widest transition-all ${
            viewMode === "heatmap"
              ? "bg-accent text-white glow-accent"
              : "text-muted-foreground hover:text-foreground"
          }`}
        >
          <Grid3x3 className="w-3 h-3" />
          Heatmap
        </button>
      </div>

      {/* Content */}
      {loading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {[1, 2, 3, 4, 5, 6].map((i) => (
            <div
              key={i}
              className="h-64 glass-card rounded-2xl animate-pulse"
            />
          ))}
        </div>
      ) : filteredCorridors.length === 0 ? (
        <div className="py-20 flex flex-col items-center justify-center glass-card rounded-3xl border-dashed">
          <AlertCircle className="w-12 h-12 text-muted-foreground/30 mb-4" />
          <p className="text-sm font-mono text-muted-foreground uppercase tracking-widest">
            No matching corridors detected
          </p>
        </div>
      ) : viewMode === "heatmap" ? (
        <div className="glass-card rounded-3xl p-8">
          <div className="mb-8">
            <h2 className="text-xl font-black tracking-tight uppercase italic mb-2">
              Corridor Health Matrix
            </h2>
            <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider">
              System health distribution across network pairs
            </p>
          </div>
          <CorridorHeatmap corridors={filteredCorridors} />
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {paginatedCorridors.map((corridor) => (
            <div
              key={corridor.id}
              className="group glass-card rounded-2xl p-6 border border-border/50 hover:border-accent/30 transition-all duration-300"
            >
              <Link href={`/corridors/${corridor.id}`}>
                {/* Header */}
                <div className="flex items-start justify-between mb-6">
                  <div className="flex-1 min-w-0">
                    <h2 className="text-xl font-bold tracking-tight text-foreground group-hover:text-accent transition-colors truncate">
                      {corridor.source_asset}{" "}
                      <span className="text-muted-foreground text-xs mx-1">
                        /
                      </span>{" "}
                      {corridor.destination_asset}
                    </h2>
                    <p className="text-[10px] font-mono text-muted-foreground/50 uppercase tracking-tighter mt-1 truncate">
                      ID: {corridor.id}
                    </p>
                  </div>
                  <div className="flex flex-col items-end gap-3">
                    <div className="flex items-center gap-2 bg-green-500/10 px-2 py-1 rounded-md border border-green-500/20">
                      <CheckCircle2 className="w-3 h-3 text-green-500" />
                      <span className="text-xs font-mono font-bold text-green-500">
                        {corridor.success_rate.toFixed(1)}%
                      </span>
                    </div>
                  </div>
                </div>

                {/* Health Radial Area */}
                <div className="mb-6 p-4 rounded-xl bg-slate-900/30 border border-white/5">
                  <div className="flex justify-between items-center mb-2">
                    <span className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest">
                      Health Score
                    </span>
                    <span
                      className={`text-sm font-mono font-black ${
                        corridor.health_score >= 90
                          ? "text-green-400"
                          : corridor.health_score >= 75
                            ? "text-yellow-400"
                            : "text-red-400"
                      }`}
                    >
                      {corridor.health_score.toFixed(0)}
                    </span>
                  </div>
                  <div className="h-1 w-full bg-white/5 rounded-full overflow-hidden">
                    <div
                      className={`h-full transition-all duration-1000 ${
                        corridor.health_score >= 90
                          ? "bg-green-500"
                          : corridor.health_score >= 75
                            ? "bg-yellow-500"
                            : "bg-red-500"
                      }`}
                      style={{ width: `${corridor.health_score}%` }}
                    />
                  </div>
                </div>

                {/* Metrics */}
                <div className="space-y-3">
                  <div className="flex justify-between items-center text-[10px] font-mono uppercase tracking-tighter">
                    <span className="text-muted-foreground">
                      Settlement Time
                    </span>
                    <span className="text-accent font-bold">
                      {corridor.average_latency_ms.toFixed(0)}ms
                    </span>
                  </div>
                  <div className="flex justify-between items-center text-[10px] font-mono uppercase tracking-tighter">
                    <span className="text-muted-foreground">
                      Liquidity Depth
                    </span>
                    <span className="text-foreground font-bold">
                      {new Intl.NumberFormat("en-US", {
                        style: "currency",
                        currency: "USD",
                        notation: "compact",
                      }).format(corridor.liquidity_depth_usd)}
                    </span>
                  </div>
                  <div className="flex justify-between items-center text-[10px] font-mono uppercase tracking-tighter">
                    <span className="text-muted-foreground">24h Vol</span>
                    <span className="text-foreground font-bold">
                      {new Intl.NumberFormat("en-US", {
                        style: "currency",
                        currency: "USD",
                        notation: "compact",
                      }).format(corridor.liquidity_volume_24h_usd)}
                    </span>
                  </div>
                </div>

                <div className="mt-6 flex justify-end opacity-0 group-hover:opacity-100 transition-opacity">
                  <ArrowRight className="w-4 h-4 text-accent animate-bounce-x" />
                </div>
              </Link>
            </div>
          ))}
        </div>
      )}

      {/* Pagination */}
      <div className="flex flex-col sm:flex-row items-center justify-between gap-4 glass-card rounded-2xl p-6 border border-border/30">
        <div className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest">
          Telemetry Feed: Viewing {startIndex + 1}-
          {Math.min(endIndex, filteredCorridors.length)} of{" "}
          {filteredCorridors.length} Nodes
        </div>
        <DataTablePagination
          currentPage={currentPage}
          pageSize={pageSize}
          totalItems={filteredCorridors.length}
          onPageChange={onPageChange}
          onPageSizeChange={onPageSizeChange}
        />
      </div>
    </div>
  );
}

export default function CorridorsPage() {
  return (
    <Suspense
      fallback={
        <div className="flex h-[80vh] items-center justify-center">
          <div className="text-sm font-mono text-accent animate-pulse uppercase tracking-widest italic">
            Syncing Satellite Routes... // 404-X
          </div>
        </div>
      }
    >
      <CorridorsPageContent />
    </Suspense>
  );
}
