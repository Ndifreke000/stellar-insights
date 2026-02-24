"use client";

import React, { useMemo, useState } from "react";
import { CorridorMetrics } from "@/lib/api";
import {
  TrendingUp,
  Droplets,
  Clock,
  CheckCircle2,
  Maximize2,
  Activity,
} from "lucide-react";
import { Link } from "@/i18n/navigation";

interface CorridorHeatmapProps {
  corridors: CorridorMetrics[];
}

interface HeatmapCell {
  sourceAsset: string;
  destinationAsset: string;
  healthScore: number;
  corridorData: CorridorMetrics;
}

interface TooltipData extends HeatmapCell {
  x: number;
  y: number;
}

export type HeatmapMetric = "health" | "success_rate" | "volume" | "latency";

export const CorridorHeatmap: React.FC<CorridorHeatmapProps> = ({
  corridors,
}) => {
  const [tooltipData, setTooltipData] = useState<TooltipData | null>(null);
  const [activeMetric, setActiveMetric] = useState<HeatmapMetric>("health");

  // Transform corridor data into matrix structure
  const { matrix, sourceAssets, destinationAssets } = useMemo(() => {
    // Extract unique assets
    const sources = Array.from(
      new Set(corridors.map((c) => c.source_asset)),
    ).sort();
    const destinations = Array.from(
      new Set(corridors.map((c) => c.destination_asset)),
    ).sort();

    // Create matrix map for O(1) lookup
    const matrixMap = new Map<string, HeatmapCell>();
    corridors.forEach((corridor) => {
      const key = `${corridor.source_asset}-${corridor.destination_asset}`;
      matrixMap.set(key, {
        sourceAsset: corridor.source_asset,
        destinationAsset: corridor.destination_asset,
        healthScore: corridor.health_score,
        corridorData: corridor,
      });
    });

    return {
      matrix: matrixMap,
      sourceAssets: sources,
      destinationAssets: destinations,
    };
  }, [corridors]);

  // Calculate min/max for volume and latency dynamically
  const { maxVolume, maxLatency, minLatency } = useMemo(() => {
    let maxV = 0;
    let maxL = 0;
    let minL = Infinity;
    corridors.forEach((c) => {
      if (c.liquidity_volume_24h_usd > maxV) maxV = c.liquidity_volume_24h_usd;
      if (c.average_latency_ms > maxL) maxL = c.average_latency_ms;
      if (c.average_latency_ms < minL) minL = c.average_latency_ms;
    });
    return {
      maxVolume: maxV || 1,
      maxLatency: maxL || 1,
      minLatency: minL === Infinity ? 0 : minL,
    };
  }, [corridors]);

  // Get color based on the selected metric
  const getCellColor = (corridor: CorridorMetrics): string => {
    switch (activeMetric) {
      case "health": {
        const score = corridor.health_score;
        if (score >= 95) return "bg-green-500";
        if (score >= 90) return "bg-green-400";
        if (score >= 85) return "bg-lime-400";
        if (score >= 80) return "bg-yellow-400";
        if (score >= 75) return "bg-orange-400";
        if (score >= 70) return "bg-orange-500";
        return "bg-red-500";
      }
      case "success_rate": {
        const rate = corridor.success_rate;
        if (rate >= 98) return "bg-green-500";
        if (rate >= 95) return "bg-green-400";
        if (rate >= 90) return "bg-lime-400";
        if (rate >= 85) return "bg-yellow-400";
        if (rate >= 80) return "bg-orange-400";
        if (rate >= 75) return "bg-orange-500";
        return "bg-red-500";
      }
      case "volume": {
        // Logarithmic scale approach or linear percentage
        const vol = corridor.liquidity_volume_24h_usd;
        const pct = vol / maxVolume;
        if (pct >= 0.8) return "bg-green-500";
        if (pct >= 0.6) return "bg-green-400";
        if (pct >= 0.4) return "bg-lime-400";
        if (pct >= 0.2) return "bg-yellow-400";
        if (pct >= 0.1) return "bg-orange-400";
        if (pct >= 0.05) return "bg-orange-500";
        return "bg-red-500"; // Low volume = red/poor performance
      }
      case "latency": {
        const lat = corridor.average_latency_ms;
        // Lower latency is better (green). Higher is worse (red).
        // Let's assume baseline good is close to minLatency.
        // If range is tight, we can just use standard thresholds or percentage of max.
        if (lat <= 1000) return "bg-green-500";
        if (lat <= 2000) return "bg-green-400";
        if (lat <= 3000) return "bg-lime-400";
        if (lat <= 4000) return "bg-yellow-400";
        if (lat <= 5000) return "bg-orange-400";
        if (lat <= 8000) return "bg-orange-500";
        return "bg-red-500";
      }
    }
  };

  const getCellOpacity = (corridor: CorridorMetrics): string => {
    // Opacity can just give a slight variation, we can use the same logic or just return 100 for simplicity now that we have strong colors.
    // For visual distinction, let's keep it simple: fully opaque.
    return "opacity-100";
  };

  const getMetricDisplayValue = (corridor: CorridorMetrics): string => {
    switch (activeMetric) {
      case "health":
        return corridor.health_score.toFixed(0);
      case "success_rate":
        return `${corridor.success_rate.toFixed(0)}%`;
      case "volume":
        if (corridor.liquidity_volume_24h_usd >= 1000000)
          return `${(corridor.liquidity_volume_24h_usd / 1000000).toFixed(1)}M`;
        if (corridor.liquidity_volume_24h_usd >= 1000)
          return `${(corridor.liquidity_volume_24h_usd / 1000).toFixed(0)}K`;
        return `${Math.floor(corridor.liquidity_volume_24h_usd)}`;
      case "latency":
        return `${(corridor.average_latency_ms / 1000).toFixed(1)}s`;
    }
  };

  // Format large numbers
  const formatCurrency = (value: number): string => {
    if (value >= 1000000) return `$${(value / 1000000).toFixed(2)}M`;
    if (value >= 1000) return `$${(value / 1000).toFixed(0)}K`;
    return `$${value.toFixed(0)}`;
  };

  // Handle mouse enter on cell
  const handleCellHover = (
    cell: HeatmapCell | null,
    event?: React.MouseEvent<HTMLDivElement>,
  ) => {
    if (cell && event) {
      const rect = event.currentTarget.getBoundingClientRect();
      setTooltipData({
        ...cell,
        x: rect.left + rect.width / 2,
        y: rect.top,
      });
    } else {
      setTooltipData(null);
    }
  };

  // Handle touch for mobile
  const handleCellTouch = (
    cell: HeatmapCell | null,
    event: React.TouchEvent<HTMLDivElement>,
  ) => {
    event.preventDefault();
    if (cell) {
      const rect = event.currentTarget.getBoundingClientRect();
      setTooltipData({
        ...cell,
        x: rect.left + rect.width / 2,
        y: rect.top,
      });
    }
  };

  // Calculate responsive cell size
  const cellSize = useMemo(() => {
    const maxAssets = Math.max(sourceAssets.length, destinationAssets.length);
    // Mobile-first sizing with larger desktop sizes
    if (maxAssets <= 4)
      return "w-14 h-14 sm:w-24 sm:h-24 lg:w-28 lg:h-28 text-xs sm:text-base";
    if (maxAssets <= 6)
      return "w-12 h-12 sm:w-20 sm:h-20 lg:w-24 lg:h-24 text-[10px] sm:text-sm";
    if (maxAssets <= 10)
      return "w-10 h-10 sm:w-16 sm:h-16 lg:w-20 lg:h-20 text-[9px] sm:text-xs";
    return "w-8 h-8 sm:w-14 sm:h-14 lg:w-16 lg:h-16 text-[8px] sm:text-xs";
  }, [sourceAssets.length, destinationAssets.length]);

  const labelSize = useMemo(() => {
    const maxAssets = Math.max(sourceAssets.length, destinationAssets.length);
    // Mobile-first sizing with larger desktop sizes
    if (maxAssets <= 4)
      return "w-14 h-14 sm:w-24 sm:h-24 lg:w-28 lg:h-28 text-xs sm:text-base";
    if (maxAssets <= 6)
      return "w-12 h-12 sm:w-20 sm:h-20 lg:w-24 lg:h-24 text-[10px] sm:text-sm";
    if (maxAssets <= 10)
      return "w-10 h-10 sm:w-16 sm:h-16 lg:w-20 lg:h-20 text-[9px] sm:text-xs";
    return "w-8 h-8 sm:w-14 sm:h-14 lg:w-16 lg:h-16 text-[8px] sm:text-xs";
  }, [sourceAssets.length, destinationAssets.length]);

  if (corridors.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 bg-gray-50 dark:bg-slate-800 rounded-lg">
        <p className="text-muted-foreground dark:text-muted-foreground">
          No corridor data available
        </p>
      </div>
    );
  }

  return (
    <div className="relative">
      {/* Mobile Info Banner */}
      <div className="mb-4 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg sm:hidden">
        <p className="text-xs text-blue-700 dark:text-blue-300 flex items-center gap-2">
          <Maximize2 className="w-4 h-4" />
          Pinch to zoom or scroll horizontally to explore the heatmap
        </p>
      </div>

      {/* Controls & Legend */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 mb-4 px-2 sm:px-0">
        {/* Metric Selector */}
        <div className="flex bg-slate-100 dark:bg-slate-900/50 p-1 rounded-xl border border-border/50">
          <button
            onClick={() => setActiveMetric("health")}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-bold transition-all ${activeMetric === "health" ? "bg-accent text-white shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
          >
            <Activity className="w-3.5 h-3.5" />
            <span className="hidden sm:inline">Health</span>
          </button>
          <button
            onClick={() => setActiveMetric("success_rate")}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-bold transition-all ${activeMetric === "success_rate" ? "bg-accent text-white shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
          >
            <CheckCircle2 className="w-3.5 h-3.5" />
            <span className="hidden sm:inline">Success</span>
          </button>
          <button
            onClick={() => setActiveMetric("volume")}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-bold transition-all ${activeMetric === "volume" ? "bg-accent text-white shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
          >
            <TrendingUp className="w-3.5 h-3.5" />
            <span className="hidden sm:inline">Volume</span>
          </button>
          <button
            onClick={() => setActiveMetric("latency")}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-bold transition-all ${activeMetric === "latency" ? "bg-accent text-white shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
          >
            <Clock className="w-3.5 h-3.5" />
            <span className="hidden sm:inline">Latency</span>
          </button>
        </div>

        {/* Legend */}
        <div className="flex items-center gap-2 sm:gap-4">
          <span className="text-xs font-medium text-gray-700 dark:text-gray-300">
            Legend:
          </span>
          <div className="flex items-center gap-1">
            <div className="w-6 h-3 sm:w-8 sm:h-4 bg-red-500 rounded-l"></div>
            <div className="w-6 h-3 sm:w-8 sm:h-4 bg-orange-500"></div>
            <div className="w-6 h-3 sm:w-8 sm:h-4 bg-yellow-400"></div>
            <div className="w-6 h-3 sm:w-8 sm:h-4 bg-lime-400"></div>
            <div className="w-6 h-3 sm:w-8 sm:h-4 bg-green-400"></div>
            <div className="w-6 h-3 sm:w-8 sm:h-4 bg-green-500 rounded-r"></div>
          </div>
          <div className="flex gap-2 text-[10px] sm:text-xs text-muted-foreground dark:text-muted-foreground">
            <span>Poor</span>
            <span>Good</span>
          </div>
        </div>
      </div>

      {/* Heatmap Container */}
      <div className="overflow-x-auto overflow-y-auto pb-4 -mx-2 sm:mx-0 touch-pan-x touch-pan-y">
        <div className="inline-block min-w-full px-2 sm:px-0">
          {/* Heatmap Matrix */}
          <div className="flex">
            {/* Y-axis labels (Destination Assets) */}
            <div className="flex flex-col sticky left-0 z-10 bg-white dark:bg-slate-800">
              <div className={labelSize}></div> {/* Empty corner cell */}
              {destinationAssets.map((destAsset) => (
                <div
                  key={`label-dest-${destAsset}`}
                  className={`${labelSize} flex items-center justify-end pr-2 sm:pr-4 lg:pr-6 font-semibold text-gray-700 dark:text-gray-300`}
                >
                  {destAsset}
                </div>
              ))}
            </div>

            {/* Matrix cells */}
            <div className="flex flex-col">
              {/* X-axis labels (Source Assets) */}
              <div className="flex">
                {sourceAssets.map((sourceAsset) => (
                  <div
                    key={`label-source-${sourceAsset}`}
                    className={`${labelSize} flex items-end justify-center pb-1 sm:pb-3 lg:pb-4 font-semibold text-gray-700 dark:text-gray-300`}
                  >
                    <span className="transform -rotate-45 origin-bottom-left whitespace-nowrap text-[9px] sm:text-sm lg:text-base">
                      {sourceAsset}
                    </span>
                  </div>
                ))}
              </div>

              {/* Data cells */}
              {destinationAssets.map((destAsset) => (
                <div key={`row-${destAsset}`} className="flex">
                  {sourceAssets.map((sourceAsset) => {
                    const key = `${sourceAsset}-${destAsset}`;
                    const cell = matrix.get(key);

                    return (
                      <div
                        key={key}
                        className={`${cellSize} relative group`}
                        onMouseEnter={(e) =>
                          cell ? handleCellHover(cell, e) : null
                        }
                        onMouseLeave={() => handleCellHover(null)}
                        onTouchStart={(e) =>
                          cell ? handleCellTouch(cell, e) : null
                        }
                        onTouchEnd={() => {
                          // Keep tooltip visible for a moment on mobile
                          setTimeout(() => setTooltipData(null), 2000);
                        }}
                      >
                        {cell ? (
                          <Link
                            href={`/corridors/${cell.corridorData.id}`}
                            passHref
                            legacyBehavior
                          >
                            <a
                              className={`w-full h-full flex items-center justify-center border border-gray-200 dark:border-slate-600 ${getCellColor(
                                cell.corridorData,
                              )} ${getCellOpacity(
                                cell.corridorData,
                              )} active:ring-2 sm:hover:ring-2 hover:ring-blue-500 hover:z-10 transition-all cursor-pointer rounded-sm`}
                            >
                              <span className="font-bold text-white drop-shadow-md text-[10px] sm:text-xs tracking-tighter">
                                {getMetricDisplayValue(cell.corridorData)}
                              </span>
                            </a>
                          </Link>
                        ) : (
                          <div className="w-full h-full bg-gray-100 dark:bg-slate-700 border border-gray-200 dark:border-slate-600 rounded-sm"></div>
                        )}
                      </div>
                    );
                  })}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Tooltip */}
      {tooltipData && (
        <div
          className="fixed z-50 pointer-events-none"
          style={{
            left: `${Math.min(Math.max(tooltipData.x, 160), window.innerWidth - 160)}px`,
            top: `${tooltipData.y - 10}px`,
            transform: "translate(-50%, -100%)",
          }}
        >
          <div className="bg-white dark:bg-slate-800 border-2 border-blue-500 rounded-lg shadow-2xl p-3 sm:p-4 min-w-[260px] sm:min-w-[280px] max-w-[90vw] sm:max-w-[320px]">
            {/* Header */}
            <div className="mb-2 sm:mb-3 pb-2 sm:pb-3 border-b border-gray-200 dark:border-slate-700">
              <h3 className="font-bold text-gray-900 dark:text-white text-sm sm:text-base mb-1">
                {tooltipData.sourceAsset} → {tooltipData.destinationAsset}
              </h3>
              <div className="flex items-center gap-2 flex-wrap">
                <div
                  className={`px-2 py-1 rounded text-xs font-bold text-white ${getCellColor(
                    tooltipData.corridorData,
                  )}`}
                >
                  Health: {tooltipData.healthScore.toFixed(1)}
                </div>
                <div className="text-[10px] sm:text-xs text-muted-foreground dark:text-muted-foreground">
                  {tooltipData.corridorData.id}
                </div>
              </div>
            </div>

            {/* Metrics Grid */}
            <div className="space-y-1.5 sm:space-y-2">
              {/* Success Rate */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 sm:gap-2 text-muted-foreground dark:text-muted-foreground">
                  <CheckCircle2 className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
                  <span className="text-xs sm:text-sm">Success Rate</span>
                </div>
                <span className="font-semibold text-green-600 dark:text-green-400 text-xs sm:text-sm">
                  {tooltipData.corridorData.success_rate.toFixed(1)}%
                </span>
              </div>

              {/* Latency */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 sm:gap-2 text-muted-foreground dark:text-muted-foreground">
                  <Clock className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
                  <span className="text-xs sm:text-sm">Avg Latency</span>
                </div>
                <span className="font-semibold text-blue-600 dark:text-link-primary text-xs sm:text-sm">
                  {tooltipData.corridorData.average_latency_ms.toFixed(0)}ms
                </span>
              </div>

              {/* Liquidity Depth */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 sm:gap-2 text-muted-foreground dark:text-muted-foreground">
                  <Droplets className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
                  <span className="text-xs sm:text-sm">Liquidity</span>
                </div>
                <span className="font-semibold text-purple-600 dark:text-purple-400 text-xs sm:text-sm">
                  {formatCurrency(tooltipData.corridorData.liquidity_depth_usd)}
                </span>
              </div>

              {/* 24h Volume */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 sm:gap-2 text-muted-foreground dark:text-muted-foreground">
                  <TrendingUp className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
                  <span className="text-xs sm:text-sm">24h Volume</span>
                </div>
                <span className="font-semibold text-amber-600 dark:text-amber-400 text-xs sm:text-sm">
                  {formatCurrency(
                    tooltipData.corridorData.liquidity_volume_24h_usd,
                  )}
                </span>
              </div>

              {/* Payment Stats */}
              <div className="pt-1.5 sm:pt-2 mt-1.5 sm:mt-2 border-t border-gray-200 dark:border-slate-700">
                <div className="flex justify-between text-[10px] sm:text-xs text-muted-foreground dark:text-muted-foreground mb-1">
                  <span>
                    {tooltipData.corridorData.successful_payments.toLocaleString()}{" "}
                    successful
                  </span>
                  <span>
                    {tooltipData.corridorData.failed_payments.toLocaleString()}{" "}
                    failed
                  </span>
                </div>
                <div className="w-full bg-gray-200 dark:bg-slate-600 rounded-full h-1.5 sm:h-2">
                  <div
                    className="bg-green-500 rounded-full h-full transition-all"
                    style={{
                      width: `${tooltipData.corridorData.success_rate}%`,
                    }}
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
