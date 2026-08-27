"use client";

import React, { useRef, useState } from "react";
import dynamic from "next/dynamic";
import { Download, ImageDown, FileImage, CheckCircle2, Loader2 } from "lucide-react";
import { exportChart, ExportFormat } from "@/lib/chart-export";
import { logger } from "@/lib/logger";

const ChartSkeleton = () => (
  <div className="h-64 w-full rounded-xl bg-white/5 animate-pulse" />
);

const LiquidityChart = dynamic(
  () => import("@/components/charts/LiquidityChart").then((m) => ({ default: m.LiquidityChart })),
  { ssr: false, loading: () => <ChartSkeleton /> }
);
const TVLChart = dynamic(
  () => import("@/components/charts/TVLChart").then((m) => ({ default: m.TVLChart })),
  { ssr: false, loading: () => <ChartSkeleton /> }
);
const SettlementLatencyChart = dynamic(
  () =>
    import("@/components/charts/SettlementLatencyChart").then((m) => ({
      default: m.SettlementLatencyChart,
    })),
  { ssr: false, loading: () => <ChartSkeleton /> }
);
const TopCorridors = dynamic(
  () => import("@/components/charts/TopCorridors").then((m) => ({ default: m.TopCorridors })),
  { ssr: false, loading: () => <ChartSkeleton /> }
);

import { useFetchAnalyticsMock } from "./useFetchAnalyticsMock";

interface ChartEntry {
  id: string;
  label: string;
  ref: React.RefObject<HTMLDivElement | null>;
}

/**
 * Renders all analytics charts with individual and batch export buttons.
 * Each chart wrapper div is wrapped in a ref so we can capture it with
 * html-to-image. Batch export downloads all selected charts sequentially.
 */
export function ChartExportGallery() {
  const { metrics, loading } = useFetchAnalyticsMock();

  const liquidityRef = useRef<HTMLDivElement>(null);
  const tvlRef = useRef<HTMLDivElement>(null);
  const latencyRef = useRef<HTMLDivElement>(null);
  const corridorsRef = useRef<HTMLDivElement>(null);

  const [selected, setSelected] = useState<Set<string>>(
    new Set(["liquidity", "tvl", "latency", "corridors"])
  );
  const [batchFormat, setBatchFormat] = useState<ExportFormat>("png");
  const [isBatchExporting, setIsBatchExporting] = useState(false);
  const [exportedCharts, setExportedCharts] = useState<Set<string>>(new Set());

  const charts: ChartEntry[] = [
    { id: "liquidity", label: "Liquidity Over Time", ref: liquidityRef },
    { id: "tvl", label: "Total Value Locked", ref: tvlRef },
    { id: "latency", label: "Settlement Latency", ref: latencyRef },
    { id: "corridors", label: "Top Corridors", ref: corridorsRef },
  ];

  const toggleSelect = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleBatchExport = async () => {
    setIsBatchExporting(true);
    setExportedCharts(new Set());
    const toExport = charts.filter((c) => selected.has(c.id));

    for (const chart of toExport) {
      if (!chart.ref.current) continue;
      try {
        const filename = `${chart.label.toLowerCase().replace(/\s+/g, "-")}-${new Date()
          .toISOString()
          .split("T")[0]}`;
        await exportChart(chart.ref.current, filename, batchFormat);
        setExportedCharts((prev) => new Set(prev).add(chart.id));
        // Small delay so the browser doesn't batch all downloads at once
        await new Promise((r) => setTimeout(r, 400));
      } catch (err) {
        logger.error(`Failed to export chart "${chart.label}"`, err);
      }
    }

    setIsBatchExporting(false);
  };

  return (
    <div className="space-y-8">
      {/* Batch export toolbar */}
      <div className="glass-card rounded-2xl p-5 flex flex-col sm:flex-row sm:items-center gap-4 justify-between">
        <div>
          <h3 className="text-sm font-bold uppercase tracking-wider text-foreground">
            Batch Chart Export
          </h3>
          <p className="text-[10px] font-mono text-muted-foreground mt-0.5">
            Select charts below, then download all as PNG or SVG.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          {/* Format selector */}
          <div className="flex items-center gap-2 text-[10px] font-mono uppercase tracking-wider text-muted-foreground">
            Format:
            {(["png", "svg"] as ExportFormat[]).map((fmt) => (
              <button
                key={fmt}
                onClick={() => setBatchFormat(fmt)}
                className={`px-2.5 py-1 rounded-lg border transition-all ${
                  batchFormat === fmt
                    ? "border-accent/50 bg-accent/10 text-accent"
                    : "border-white/10 text-muted-foreground hover:border-white/20"
                }`}
              >
                {fmt.toUpperCase()}
              </button>
            ))}
          </div>

          <button
            onClick={handleBatchExport}
            disabled={isBatchExporting || selected.size === 0}
            className="
              flex items-center gap-2 px-4 py-2
              text-[10px] font-mono uppercase tracking-wider
              bg-accent text-white rounded-lg
              hover:bg-accent/90 transition-colors
              disabled:opacity-50 disabled:cursor-not-allowed
            "
          >
            {isBatchExporting ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" />
            ) : (
              <Download className="w-3.5 h-3.5" aria-hidden="true" />
            )}
            {isBatchExporting
              ? "Exporting…"
              : `Export ${selected.size} Chart${selected.size !== 1 ? "s" : ""}`}
          </button>
        </div>
      </div>

      {/* Chart selection checkboxes */}
      <div className="flex flex-wrap gap-3">
        {charts.map((c) => (
          <button
            key={c.id}
            onClick={() => toggleSelect(c.id)}
            aria-pressed={selected.has(c.id)}
            className={`
              flex items-center gap-2 px-3 py-1.5 rounded-lg border
              text-[10px] font-mono uppercase tracking-wider transition-all
              ${
                selected.has(c.id)
                  ? "border-accent/40 bg-accent/10 text-accent"
                  : "border-white/10 text-muted-foreground hover:border-white/20"
              }
            `}
          >
            {exportedCharts.has(c.id) ? (
              <CheckCircle2 className="w-3 h-3 text-green-400" aria-hidden="true" />
            ) : batchFormat === "svg" ? (
              <FileImage className="w-3 h-3" aria-hidden="true" />
            ) : (
              <ImageDown className="w-3 h-3" aria-hidden="true" />
            )}
            {c.label}
          </button>
        ))}
      </div>

      {/* Charts grid */}
      {loading ? (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {[1, 2, 3, 4].map((i) => (
            <ChartSkeleton key={i} />
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div ref={liquidityRef}>
            {metrics && <LiquidityChart data={metrics.liquidity_history} />}
          </div>
          <div ref={tvlRef}>
            {metrics && <TVLChart data={metrics.tvl_history} />}
          </div>
          <div ref={latencyRef}>
            {metrics && (
              <SettlementLatencyChart data={metrics.settlement_latency_history} />
            )}
          </div>
          <div ref={corridorsRef}>
            {metrics && <TopCorridors corridors={metrics.top_corridors} />}
          </div>
        </div>
      )}
    </div>
  );
}
