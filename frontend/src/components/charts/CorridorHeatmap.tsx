"use client";

import React, { useMemo, useState } from "react";
import { CorridorMetrics } from "@/lib/api";
import { TrendingUp, Droplets, Clock, CheckCircle2, Maximize2 } from "lucide-react";

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

export const CorridorHeatmap: React.FC<CorridorHeatmapProps> = ({
  corridors,
}) => {
  const [tooltipData, setTooltipData] = useState<TooltipData | null>(null);

  // Transform corridor data into matrix structure
  const { matrix, sourceAssets, destinationAssets } = useMemo(() => {
    // Extract unique assets
    const sources = Array.from(
      new Set(corridors.map((c) => c.source_asset))
    ).sort();
    const destinations = Array.from(
      new Set(corridors.map((c) => c.destination_asset))
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

  // Get color based on health score
  const getHealthColor = (score: number): string => {
    if (score >= 95) return "bg-green-500";
    if (score >= 90) return "bg-green-400";
    if (score >= 85) return "bg-lime-400";
    if (score >= 80) return "bg-yellow-400";
    if (score >= 75) return "bg-orange-400";
    if (score >= 70) return "bg-orange-500";
    return "bg-red-500";
  };

  // Get opacity based on health score for better visual distinction
  const getOpacity = (score: number): string => {
    if (score >= 90) return "opacity-100";
    if (score >= 80) return "opacity-90";
    if (score >= 70) return "opacity-80";
    return "opacity-70";
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
    event?: React.MouseEvent<HTMLDivElement>
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
  const handleCellTouch = (cell: HeatmapCell | null, event: React.TouchEvent<HTMLDivElement>) => {
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
    if (maxAssets <= 4) return "w-14 h-14 sm:w-24 sm:h-24 lg:w-28 lg:h-28 text-xs sm:text-base";
    if (maxAssets <= 6) return "w-12 h-12 sm:w-20 sm:h-20 lg:w-24 lg:h-24 text-[10px] sm:text-sm";
    if (maxAssets <= 10) return "w-10 h-10 sm:w-16 sm:h-16 lg:w-20 lg:h-20 text-[9px] sm:text-xs";
    return "w-8 h-8 sm:w-14 sm:h-14 lg:w-16 lg:h-16 text-[8px] sm:text-xs";
  }, [sourceAssets.length, destinationAssets.length]);

  const labelSize = useMemo(() => {
    const maxAssets = Math.max(sourceAssets.length, destinationAssets.length);
    // Mobile-first sizing with larger desktop sizes
    if (maxAssets <= 4) return "w-14 h-14 sm:w-24 sm:h-24 lg:w-28 lg:h-28 text-xs sm:text-base";
    if (maxAssets <= 6) return "w-12 h-12 sm:w-20 sm:h-20 lg:w-24 lg:h-24 text-[10px] sm:text-sm";
    if (maxAssets <= 10) return "w-10 h-10 sm:w-16 sm:h-16 lg:w-20 lg:h-20 text-[9px] sm:text-xs";
    return "w-8 h-8 sm:w-14 sm:h-14 lg:w-16 lg:h-16 text-[8px] sm:text-xs";
  }, [sourceAssets.length, destinationAssets.length]);

  if (corridors.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 bg-gray-50 dark:bg-slate-800 rounded-lg">
        <p className="text-gray-500 dark:text-gray-400">
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

      {/* Heatmap Container */}
      <div className="overflow-x-auto overflow-y-auto pb-4 -mx-2 sm:mx-0 touch-pan-x touch-pan-y">
        <div className="inline-block min-w-full px-2 sm:px-0">
          {/* Legend */}
          <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 mb-4 px-2 sm:px-4">
            <span className="text-xs sm:text-sm font-medium text-gray-700 dark:text-gray-300">
              Health Score Legend:
            </span>
            <div className="flex items-center gap-2 sm:gap-4">
              <div className="flex items-center gap-1">
                <div className="w-6 h-3 sm:w-8 sm:h-4 bg-red-500 rounded-l"></div>
                <div className="w-6 h-3 sm:w-8 sm:h-4 bg-orange-500"></div>
                <div className="w-6 h-3 sm:w-8 sm:h-4 bg-yellow-400"></div>
                <div className="w-6 h-3 sm:w-8 sm:h-4 bg-lime-400"></div>
                <div className="w-6 h-3 sm:w-8 sm:h-4 bg-green-400"></div>
                <div className="w-6 h-3 sm:w-8 sm:h-4 bg-green-500 rounded-r"></div>
              </div>
              <div className="flex gap-2 text-[10px] sm:text-xs text-gray-600 dark:text-gray-400">
                <span>Low</span>
                <span>High</span>
              </div>
            </div>
          </div>

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
                        onTouchStart={(e) => cell ? handleCellTouch(cell, e) : null}
                        onTouchEnd={() => {
                          // Keep tooltip visible for a moment on mobile
                          setTimeout(() => setTooltipData(null), 2000);
                        }}
                      >
                        {cell ? (
                          <div
                            className={`w-full h-full flex items-center justify-center border border-gray-200 dark:border-slate-600 ${getHealthColor(
                              cell.healthScore
                            )} ${getOpacity(
                              cell.healthScore
                            )} active:ring-2 sm:hover:ring-2 hover:ring-blue-500 hover:z-10 transition-all cursor-pointer rounded-sm`}
                          >
                            <span className="font-bold text-white drop-shadow-md">
                              {cell.healthScore.toFixed(0)}
                            </span>
                          </div>
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
                  className={`px-2 py-1 rounded text-xs font-bold text-white ${getHealthColor(
                    tooltipData.healthScore
                  )}`}
                >
                  Health: {tooltipData.healthScore.toFixed(1)}
                </div>
                <div className="text-[10px] sm:text-xs text-gray-600 dark:text-gray-400">
                  {tooltipData.corridorData.id}
                </div>
              </div>
            </div>

            {/* Metrics Grid */}
            <div className="space-y-1.5 sm:space-y-2">
              {/* Success Rate */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 sm:gap-2 text-gray-600 dark:text-gray-400">
                  <CheckCircle2 className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
                  <span className="text-xs sm:text-sm">Success Rate</span>
                </div>
                <span className="font-semibold text-green-600 dark:text-green-400 text-xs sm:text-sm">
                  {tooltipData.corridorData.success_rate.toFixed(1)}%
                </span>
              </div>

              {/* Latency */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 sm:gap-2 text-gray-600 dark:text-gray-400">
                  <Clock className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
                  <span className="text-xs sm:text-sm">Avg Latency</span>
                </div>
                <span className="font-semibold text-blue-600 dark:text-blue-400 text-xs sm:text-sm">
                  {tooltipData.corridorData.average_latency_ms.toFixed(0)}ms
                </span>
              </div>

              {/* Liquidity Depth */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 sm:gap-2 text-gray-600 dark:text-gray-400">
                  <Droplets className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
                  <span className="text-xs sm:text-sm">Liquidity</span>
                </div>
                <span className="font-semibold text-purple-600 dark:text-purple-400 text-xs sm:text-sm">
                  {formatCurrency(tooltipData.corridorData.liquidity_depth_usd)}
                </span>
              </div>

              {/* 24h Volume */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 sm:gap-2 text-gray-600 dark:text-gray-400">
                  <TrendingUp className="w-3.5 h-3.5 sm:w-4 sm:h-4" />
                  <span className="text-xs sm:text-sm">24h Volume</span>
                </div>
                <span className="font-semibold text-amber-600 dark:text-amber-400 text-xs sm:text-sm">
                  {formatCurrency(
                    tooltipData.corridorData.liquidity_volume_24h_usd
                  )}
                </span>
              </div>

              {/* Payment Stats */}
              <div className="pt-1.5 sm:pt-2 mt-1.5 sm:mt-2 border-t border-gray-200 dark:border-slate-700">
                <div className="flex justify-between text-[10px] sm:text-xs text-gray-600 dark:text-gray-400 mb-1">
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