"use client";

/**
 * Virtualised, streaming corridor table powered by TanStack Virtual (Issue #2835).
 *
 * - Renders only the visible viewport slice (+ overscan) via `useVirtualizer`
 * - Stable row keys (`getItemKey: (i) => rows[i].id`)
 * - `React.memo` rows and cells with primitive positioning props so a live
 *   WebSocket patch re-renders ONLY the affected row (O(1) React commits)
 * - Server-driven sorting headers and cursor-paginated infinite scroll
 */
import React, { memo, useEffect, useRef, useCallback } from "react";
import { observeElementRect, useVirtualizer } from "@tanstack/react-virtual";
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  CheckCircle2,
  Loader2,
  Radio,
} from "lucide-react";
import { Link } from "@/i18n/navigation";
import type { CorridorMetrics } from "@/lib/api/corridors";

export type CorridorTableSortField =
  | "health_score"
  | "success_rate"
  | "liquidity";

export type SortDirection = "asc" | "desc";

export interface VirtualizedCorridorTableProps {
  rows: CorridorMetrics[];
  sortBy: CorridorTableSortField;
  sortOrder: SortDirection;
  onSortChange?: (field: CorridorTableSortField, order: SortDirection) => void;
  hasNextPage?: boolean;
  isFetchingNextPage?: boolean;
  onLoadMore?: () => void;
  totalCount?: number | null;
  lastPatchedId?: string | null;
  isLiveConnected?: boolean;
  /** Viewport height in pixels (default: 560). */
  height?: number;
  /** Fixed row height in pixels for smooth 60fps virtualisation (default: 56). */
  rowHeight?: number;
  /** Optional hook for React Profiler / render-count verification in tests. */
  onRowRender?: (corridorId: string) => void;
}

// Module-level formatters prevent allocating new Intl instances per cell/frame.
const USD_COMPACT = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  notation: "compact",
  maximumFractionDigits: 2,
});

// ── Memoized Cells ────────────────────────────────────────────────────────────

const CorridorPairCell = memo(function CorridorPairCell({
  id,
  sourceAsset,
  destinationAsset,
}: {
  id: string;
  sourceAsset: string;
  destinationAsset: string;
}) {
  return (
    <div role="cell" className="min-w-0 flex-[2.2] px-4 truncate">
      <Link
        href={`/corridors/${id}`}
        className="font-bold text-sm text-foreground hover:text-accent transition-colors truncate block"
      >
        {sourceAsset} <span className="text-muted-foreground mx-0.5">→</span>{" "}
        {destinationAsset}
      </Link>
      <div className="text-[10px] font-mono text-muted-foreground/70 truncate">
        {id}
      </div>
    </div>
  );
});

const SuccessRateCell = memo(function SuccessRateCell({
  successRate,
}: {
  successRate: number;
}) {
  const tone =
    successRate >= 95
      ? "text-emerald-400 bg-emerald-500/10 border-emerald-500/20"
      : successRate >= 85
        ? "text-amber-400 bg-amber-500/10 border-amber-500/20"
        : "text-red-400 bg-red-500/10 border-red-500/20";

  return (
    <div role="cell" className="flex-1 px-3 flex items-center">
      <span
        className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-md border text-xs font-mono font-bold ${tone}`}
      >
        <CheckCircle2 className="w-3 h-3 shrink-0" aria-hidden="true" />
        {successRate.toFixed(1)}%
      </span>
    </div>
  );
});

const HealthScoreCell = memo(function HealthScoreCell({
  healthScore,
}: {
  healthScore: number;
}) {
  const color =
    healthScore >= 90
      ? "text-emerald-400"
      : healthScore >= 75
        ? "text-amber-400"
        : "text-red-400";
  const barColor =
    healthScore >= 90
      ? "bg-emerald-500"
      : healthScore >= 75
        ? "bg-amber-500"
        : "bg-red-500";
  const widthPct = Math.min(100, Math.max(0, healthScore));

  return (
    <div role="cell" className="flex-1 px-3 flex flex-col justify-center gap-1">
      <span className={`text-xs font-mono font-black ${color}`}>
        {healthScore.toFixed(1)}
      </span>
      <div className="h-1 w-20 bg-white/10 rounded-full overflow-hidden">
        <div
          className={`h-full ${barColor}`}
          style={{ width: `${widthPct}%` }}
        />
      </div>
    </div>
  );
});

const LatencyCell = memo(function LatencyCell({
  latencyMs,
}: {
  latencyMs: number;
}) {
  return (
    <div
      role="cell"
      className="flex-1 px-3 text-xs font-mono text-accent font-semibold"
    >
      {latencyMs.toFixed(0)}ms
    </div>
  );
});

const UsdCell = memo(function UsdCell({ amount }: { amount: number }) {
  return (
    <div
      role="cell"
      className="flex-1 px-3 text-xs font-mono text-foreground font-semibold"
    >
      {USD_COMPACT.format(amount)}
    </div>
  );
});

const AttemptsCell = memo(function AttemptsCell({
  totalAttempts,
}: {
  totalAttempts: number;
}) {
  return (
    <div
      role="cell"
      className="flex-[0.8] px-3 text-xs font-mono text-muted-foreground"
    >
      {totalAttempts.toLocaleString("en-US")}
    </div>
  );
});

// ── Memoized Virtual Row ──────────────────────────────────────────────────────

interface CorridorRowProps {
  corridor: CorridorMetrics;
  virtualStart: number;
  rowHeight: number;
  rowIndex: number;
  isPatched: boolean;
  onRowRender?: (corridorId: string) => void;
}

const CorridorVirtualRow = memo(function CorridorVirtualRow({
  corridor,
  virtualStart,
  rowHeight,
  rowIndex,
  isPatched,
  onRowRender,
}: CorridorRowProps) {
  onRowRender?.(corridor.id);

  return (
    <div
      role="row"
      aria-rowindex={rowIndex + 2}
      data-testid={`corridor-row-${corridor.id}`}
      data-row-id={corridor.id}
      data-patched={isPatched ? "true" : "false"}
      style={{
        position: "absolute",
        top: 0,
        left: 0,
        width: "100%",
        height: `${rowHeight}px`,
        transform: `translateY(${virtualStart}px)`,
      }}
      className={`flex items-center border-b border-border/30 transition-colors hover:bg-slate-800/40 ${
        isPatched
          ? "bg-accent/15 ring-1 ring-inset ring-accent/40"
          : rowIndex % 2 === 0
            ? "bg-slate-950/30"
            : "bg-slate-900/20"
      }`}
    >
      <CorridorPairCell
        id={corridor.id}
        sourceAsset={corridor.source_asset}
        destinationAsset={corridor.destination_asset}
      />
      <SuccessRateCell successRate={corridor.success_rate} />
      <HealthScoreCell healthScore={corridor.health_score} />
      <LatencyCell latencyMs={corridor.average_latency_ms} />
      <UsdCell amount={corridor.liquidity_depth_usd} />
      <UsdCell amount={corridor.liquidity_volume_24h_usd} />
      <AttemptsCell totalAttempts={corridor.total_attempts} />
    </div>
  );
});

// ── Sortable Header Button ────────────────────────────────────────────────────

function SortHeaderButton({
  field,
  label,
  activeField,
  sortOrder,
  onToggle,
}: {
  field: CorridorTableSortField;
  label: string;
  activeField: CorridorTableSortField;
  sortOrder: SortDirection;
  onToggle: (field: CorridorTableSortField) => void;
}) {
  const isActive = activeField === field;
  return (
    <button
      type="button"
      onClick={() => onToggle(field)}
      aria-label={`Sort by ${label}`}
      className="inline-flex items-center gap-1 uppercase hover:text-foreground transition-colors cursor-pointer"
    >
      <span>{label}</span>
      {isActive ? (
        sortOrder === "asc" ? (
          <ArrowUp className="w-3 h-3 text-accent" aria-hidden="true" />
        ) : (
          <ArrowDown className="w-3 h-3 text-accent" aria-hidden="true" />
        )
      ) : (
        <ArrowUpDown className="w-3 h-3 opacity-40" aria-hidden="true" />
      )}
    </button>
  );
}

// ── Main Virtualised Corridor Table ───────────────────────────────────────────

export function VirtualizedCorridorTable({
  rows,
  sortBy,
  sortOrder,
  onSortChange,
  hasNextPage = false,
  isFetchingNextPage = false,
  onLoadMore,
  totalCount,
  lastPatchedId = null,
  isLiveConnected = false,
  height = 560,
  rowHeight = 56,
  onRowRender,
}: VirtualizedCorridorTableProps) {
  const scrollContainerRef = useRef<HTMLDivElement | null>(null);

  const getItemKey = useCallback(
    (index: number) => rows[index]?.id ?? String(index),
    [rows],
  );

  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollContainerRef.current,
    estimateSize: () => rowHeight,
    getItemKey,
    overscan: 10,
    initialRect: { width: 1000, height },
    observeElementRect: (instance, cb) =>
      observeElementRect(instance, (rect) => {
        cb(
          rect.height > 0
            ? rect
            : { width: rect.width || 1000, height },
        );
      }),
  });

  const virtualItems = virtualizer.getVirtualItems();
  const lastVisibleIndex =
    virtualItems.length > 0
      ? virtualItems[virtualItems.length - 1].index
      : -1;

  // Trigger cursor-paginated fetch when user scrolls near the loaded tail.
  useEffect(() => {
    if (
      lastVisibleIndex >= rows.length - 10 &&
      hasNextPage &&
      !isFetchingNextPage &&
      onLoadMore
    ) {
      onLoadMore();
    }
  }, [
    lastVisibleIndex,
    rows.length,
    hasNextPage,
    isFetchingNextPage,
    onLoadMore,
  ]);

  const handleSortToggle = useCallback(
    (field: CorridorTableSortField) => {
      if (!onSortChange) return;
      if (field === sortBy) {
        onSortChange(field, sortOrder === "desc" ? "asc" : "desc");
      } else {
        onSortChange(field, "desc");
      }
    },
    [onSortChange, sortBy, sortOrder],
  );

  const ariaRowCount = (totalCount ?? rows.length) + 1;

  return (
    <div
      className="glass-card rounded-2xl border border-border/50 overflow-hidden"
      data-testid="virtualized-corridor-table"
    >
      {/* Status bar */}
      <div className="flex flex-wrap items-center justify-between gap-2 px-4 py-2.5 border-b border-border/40 bg-slate-950/60 text-[11px] font-mono text-muted-foreground">
        <div className="flex items-center gap-2">
          <Radio
            className={`w-3.5 h-3.5 ${
              isLiveConnected ? "text-emerald-400 animate-pulse" : "text-slate-500"
            }`}
            aria-hidden="true"
          />
          <span>
            {isLiveConnected ? "LIVE STREAM" : "SNAPSHOT"} · Showing{" "}
            <strong className="text-foreground">{rows.length.toLocaleString("en-US")}</strong>
            {totalCount != null && totalCount > rows.length
              ? ` of ${totalCount.toLocaleString("en-US")}`
              : ""}{" "}
            corridors ({virtualItems.length} rendered in DOM)
          </span>
        </div>
        {isFetchingNextPage && (
          <span className="inline-flex items-center gap-1.5 text-accent">
            <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" />
            Loading next page…
          </span>
        )}
      </div>

      {/* Table container */}
      <div
        role="table"
        aria-label="Payment corridors"
        aria-rowcount={ariaRowCount}
        className="w-full"
      >
        {/* Sticky header */}
        <div
          role="rowgroup"
          className="bg-slate-900/80 border-b border-border/50 text-[10px] font-mono uppercase tracking-wider text-muted-foreground"
        >
          <div role="row" aria-rowindex={1} className="flex items-center h-11">
            <div role="columnheader" className="flex-[2.2] px-4">
              Corridor
            </div>
            <div
              role="columnheader"
              aria-sort={
                sortBy === "success_rate"
                  ? sortOrder === "asc"
                    ? "ascending"
                    : "descending"
                  : "none"
              }
              className="flex-1 px-3"
            >
              <SortHeaderButton
                field="success_rate"
                label="Success Rate"
                activeField={sortBy}
                sortOrder={sortOrder}
                onToggle={handleSortToggle}
              />
            </div>
            <div
              role="columnheader"
              aria-sort={
                sortBy === "health_score"
                  ? sortOrder === "asc"
                    ? "ascending"
                    : "descending"
                  : "none"
              }
              className="flex-1 px-3"
            >
              <SortHeaderButton
                field="health_score"
                label="Health"
                activeField={sortBy}
                sortOrder={sortOrder}
                onToggle={handleSortToggle}
              />
            </div>
            <div role="columnheader" className="flex-1 px-3">
              Latency
            </div>
            <div
              role="columnheader"
              aria-sort={
                sortBy === "liquidity"
                  ? sortOrder === "asc"
                    ? "ascending"
                    : "descending"
                  : "none"
              }
              className="flex-1 px-3"
            >
              <SortHeaderButton
                field="liquidity"
                label="Liquidity"
                activeField={sortBy}
                sortOrder={sortOrder}
                onToggle={handleSortToggle}
              />
            </div>
            <div role="columnheader" className="flex-1 px-3">
              24h Vol
            </div>
            <div role="columnheader" className="flex-[0.8] px-3">
              Attempts
            </div>
          </div>
        </div>

        {/* Scrollable virtualised body */}
        <div
          ref={scrollContainerRef}
          data-testid="corridor-table-scroll-container"
          style={{ height: `${height}px` }}
          className="overflow-y-auto overflow-x-auto relative will-change-scroll"
        >
          <div
            role="rowgroup"
            style={{
              height: `${virtualizer.getTotalSize()}px`,
              width: "100%",
              position: "relative",
            }}
          >
            {virtualItems.map((virtualItem) => {
              const corridor = rows[virtualItem.index];
              if (!corridor) return null;
              return (
                <CorridorVirtualRow
                  key={corridor.id}
                  corridor={corridor}
                  virtualStart={virtualItem.start}
                  rowHeight={rowHeight}
                  rowIndex={virtualItem.index}
                  isPatched={lastPatchedId === corridor.id}
                  onRowRender={onRowRender}
                />
              );
            })}
          </div>
        </div>
      </div>

      {/* Footer / cursor pagination control */}
      {hasNextPage && onLoadMore && (
        <div className="flex items-center justify-center py-3 border-t border-border/40 bg-slate-950/50">
          <button
            type="button"
            onClick={onLoadMore}
            disabled={isFetchingNextPage}
            className="px-4 py-1.5 rounded-lg border border-accent/30 bg-accent/10 text-xs font-mono uppercase tracking-wider text-accent hover:bg-accent hover:text-white disabled:opacity-50 transition-colors"
          >
            {isFetchingNextPage ? "Loading more…" : "Load next cursor page"}
          </button>
        </div>
      )}
    </div>
  );
}
