"use client";

import { useState, useMemo, useCallback } from "react";
import { useTranslations } from "next-intl";
import dynamic from "next/dynamic";
import {
  TrendingUp,
  CheckCircle2,
  AlertCircle,
  ArrowRight,
  Download,
} from "lucide-react";
import {
  AdvancedFilterPanel,
  applyFilters,
  type AdvancedFilterState,
} from "@/components/AdvancedFilterPanel";
import { Badge } from "@/components/ui/badge";
import { SkeletonCorridorCard } from "@/components/ui/Skeleton";
import { Link } from "@/i18n/navigation";
import type { CorridorFilters, CorridorMetrics } from "@/lib/api/corridors";
import { useCorridors } from "@/lib/react-query/queries";
import { DataTablePagination } from "@/components/ui/DataTablePagination";
import { usePagination } from "@/hooks/usePagination";
import { ExportDialog } from "@/components/ExportDialog";
import {
  useUserPreferences,
  type CorridorsTimePeriod,
} from "@/contexts/UserPreferencesContext";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { BookmarkButton } from "@/components/BookmarkButton";
import { ObservedDataNotice } from "@/components/corridors/ObservedDataError";
import {
  VirtualizedCorridorTable,
  type CorridorTableSortField,
  type SortDirection,
} from "@/components/corridors/VirtualizedCorridorTable";
import {
  useRealtimeCorridors,
  type CorridorUpdate,
} from "@/hooks/useRealtimeCorridors";
import { usePatchableRows } from "@/hooks/useStreamingTableData";

// Lazy-load the heatmap — only rendered when the user switches to heatmap view.
const CorridorHeatmap = dynamic(
  () =>
    import("@/components/charts/CorridorHeatmap").then((m) => ({
      default: m.CorridorHeatmap,
    })),
  { ssr: false },
);

const USD_COMPACT = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  notation: "compact",
});

function CorridorsPageContent() {
  const t = useTranslations("corridors");
  const { prefs, setPrefs } = useUserPreferences();

  const [isExportOpen, setIsExportOpen] = useState(false);
  const [sortOrder, setSortOrder] = useState<SortDirection>("desc");
  const [cursor, setCursor] = useState<string | undefined>(undefined);
  const [previousRows, setPreviousRows] = useState<CorridorMetrics[]>([]);

  const viewMode = prefs.corridorsViewMode;
  const setViewMode = (v: typeof viewMode) =>
    setPrefs({ corridorsViewMode: v });
  const sortBy = prefs.corridorsSortBy;
  const setSortBy = (v: typeof sortBy) => {
    setPrefs({ corridorsSortBy: v });
    setCursor(undefined);
    setPreviousRows([]);
  };
  const timePeriod = prefs.corridorsTimePeriod;
  const setTimePeriod = (v: typeof timePeriod) => {
    setPrefs({ corridorsTimePeriod: v });
    setCursor(undefined);
    setPreviousRows([]);
  };

  const [filterState, setFilterState] = useState<AdvancedFilterState>({
    query: "",
    filters: [],
  });

  const handleFilterChange = useCallback((next: AdvancedFilterState) => {
    setFilterState(next);
    setCursor(undefined);
    setPreviousRows([]);
  }, []);

  // Derive server-side filter parameters from the filter bar so sorting,
  // filtering, and cursor pagination are server-driven (#2835).
  const serverFilters = useMemo<CorridorFilters>(() => {
    const filters: CorridorFilters = {
      time_period: timePeriod || undefined,
      sort_by: sortBy,
      sort_order: sortOrder,
      limit: 50,
      cursor,
    };

    if (filterState.query.trim()) {
      filters.search = filterState.query.trim();
    }

    for (const rule of filterState.filters) {
      const val = String(rule.value ?? "").trim();
      if (!val) continue;
      if (rule.field === "source_asset") {
        filters.source_asset = val;
      } else if (rule.field === "destination_asset") {
        filters.destination_asset = val;
      } else if (rule.field === "status") {
        filters.status = val;
      } else if (rule.field === "success_rate") {
        const parsed = Number.parseFloat(val);
        if (Number.isFinite(parsed)) {
          filters.success_rate_min = parsed;
        }
      }
    }

    return filters;
  }, [timePeriod, sortBy, sortOrder, cursor, filterState]);

  const corridorQuery = useCorridors(serverFilters);

  // Accumulate cursor-paginated pages in order without duplicating rows.
  const pageData = corridorQuery.data?.data;
  const nextCursor = corridorQuery.data?.pagination?.next_cursor ?? null;
  const totalServerCount = corridorQuery.data?.pagination?.total ?? null;

  const loadedCorridors = useMemo<CorridorMetrics[]>(() => {
    if (previousRows.length === 0) {
      return pageData ?? [];
    }
    if (!pageData || pageData.length === 0) {
      return previousRows;
    }
    const seen = new Set<string>();
    const all: CorridorMetrics[] = [];
    for (const row of previousRows) {
      if (seen.has(row.id)) continue;
      seen.add(row.id);
      all.push(row);
    }
    for (const row of pageData) {
      if (seen.has(row.id)) continue;
      seen.add(row.id);
      all.push(row);
    }
    return all;
  }, [previousRows, pageData]);

  // Normalized row store: live WebSocket updates patch ONLY the affected row
  // object so untouched memoized rows in VirtualizedCorridorTable never re-render.
  const {
    rows: streamingCorridors,
    patchRow,
    lastPatchedId,
  } = usePatchableRows(loadedCorridors);

  const handleRealtimePatch = useCallback(
    (update: CorridorUpdate) => {
      if (!update.corridor_key) return;
      patchRow(update.corridor_key, (current) => {
        const nextSuccess = update.success_rate ?? current.success_rate;
        const nextHealth = update.health_score ?? current.health_score;
        const nextLiquidity =
          update.liquidity_depth_usd ?? current.liquidity_depth_usd;
        const nextVolume =
          update.liquidity_volume_24h_usd ?? current.liquidity_volume_24h_usd;
        const nextLatency =
          update.average_latency_ms ?? current.average_latency_ms;
        const nextAttempts =
          update.total_transactions ?? current.total_attempts;
        const nextUpdated = update.last_updated ?? current.last_updated;

        if (
          nextSuccess === current.success_rate &&
          nextHealth === current.health_score &&
          nextLiquidity === current.liquidity_depth_usd &&
          nextVolume === current.liquidity_volume_24h_usd &&
          nextLatency === current.average_latency_ms &&
          nextAttempts === current.total_attempts &&
          nextUpdated === current.last_updated
        ) {
          return current;
        }

        return {
          ...current,
          success_rate: nextSuccess,
          health_score: nextHealth,
          liquidity_depth_usd: nextLiquidity,
          liquidity_volume_24h_usd: nextVolume,
          average_latency_ms: nextLatency,
          total_attempts: nextAttempts,
          last_updated: nextUpdated,
        };
      });
    },
    [patchRow],
  );

  // Shares the single tab-wide WebSocket singleton (#2834).
  const { isConnected: isLiveConnected } = useRealtimeCorridors({
    onCorridorUpdate: handleRealtimePatch,
  });

  // A failed fetch must not be filled with demo corridors. Cached rows, if
  // any, stay visible with a warning; otherwise the page shows an error.
  const loadFailed = corridorQuery.isError;
  const showStaleWarning = loadFailed && streamingCorridors.length > 0;
  const showErrorPanel = loadFailed && streamingCorridors.length === 0;
  const loading = corridorQuery.isPending && !loadFailed;

  // Apply any client-side range filter operators not natively represented as
  // single query params while preserving server-driven row order.
  const filteredCorridors = useMemo(() => {
    if (!filterState.query && filterState.filters.length === 0) {
      return streamingCorridors;
    }
    return applyFilters(
      streamingCorridors as unknown as Record<string, unknown>[],
      filterState,
    ) as unknown as CorridorMetrics[];
  }, [streamingCorridors, filterState]);

  const handleLoadMore = useCallback(() => {
    if (nextCursor && !corridorQuery.isFetching) {
      setPreviousRows(loadedCorridors);
      setCursor(nextCursor);
    }
  }, [nextCursor, corridorQuery.isFetching, loadedCorridors]);

  const handleTableSortChange = useCallback(
    (field: CorridorTableSortField, order: SortDirection) => {
      setPrefs({ corridorsSortBy: field });
      setSortOrder(order);
      setCursor(undefined);
      setPreviousRows([]);
    },
    [setPrefs],
  );

  const {
    currentPage,
    pageSize,
    onPageChange,
    onPageSizeChange,
    startIndex,
    endIndex,
  } = usePagination(filteredCorridors.length);

  const paginatedCorridors = filteredCorridors.slice(startIndex, endIndex);

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
            Network Routing // 02
          </div>
          <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
            <TrendingUp className="w-8 h-8 text-accent" />
            {t("title")}
          </h2>
        </div>
        <div className="flex items-center gap-3">
          <Badge
            variant="outline"
            className="text-[10px] font-mono border-accent/30 text-accent px-3 py-1 bg-accent/5"
          >
            {totalServerCount ?? filteredCorridors.length} ACTIVE_ROUTES
          </Badge>
          <Link
            href="/corridors/forecasting"
            className="flex items-center gap-2 px-4 py-2 bg-blue-500/10 border border-blue-500/20 rounded-xl text-[10px] font-bold uppercase tracking-widest text-blue-400 hover:bg-blue-500 hover:text-white transition-all"
          >
            Forecast
          </Link>
          <button
            onClick={() => setIsExportOpen(true)}
            className="flex items-center gap-2 px-4 py-2 bg-accent/10 border border-accent/20 rounded-xl text-[10px] font-bold uppercase tracking-widest text-accent hover:bg-accent hover:text-white transition-all shadow-[0_0_15px_rgba(var(--accent-rgb),0.1)] hover:shadow-accent/30"
          >
            <Download className="w-3 h-3" />
            {t("comparison.exportCsv")}
          </button>
        </div>
      </div>

      <ExportDialog
        isOpen={isExportOpen}
        onClose={() => setIsExportOpen(false)}
        type="corridors"
        title={t("title")}
      />

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-4">
        {/* Advanced Filter Panel (#2110) */}
        <div className="lg:col-span-8">
          <AdvancedFilterPanel
            fields={[
              {
                key: "source_asset",
                label: "Source Asset",
                type: "text",
                placeholder: "e.g. USDC",
              },
              {
                key: "destination_asset",
                label: "Dest Asset",
                type: "text",
                placeholder: "e.g. PHP",
              },
              {
                key: "status",
                label: "Status",
                type: "select",
                options: [
                  { value: "active", label: "Active" },
                  { value: "inactive", label: "Inactive" },
                  { value: "degraded", label: "Degraded" },
                ],
              },
              {
                key: "success_rate",
                label: "Success Rate (%)",
                type: "number",
                placeholder: "e.g. 95",
              },
              {
                key: "health_score",
                label: "Health Score",
                type: "range",
                min: 0,
                max: 100,
                step: 1,
              },
            ]}
            onChange={handleFilterChange}
            storageKey="corridors_advanced_filter"
            placeholder={t("search")}
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
            <option value="health_score">{t("sort.health")}</option>
            <option value="success_rate">{t("sort.success")}</option>
            <option value="liquidity">{t("sort.liquidity")}</option>
          </select>
        </div>
      </div>

      {showStaleWarning && (
        <ObservedDataNotice
          variant="warning"
          message="Some corridor data failed to refresh. Showing the last observed results only."
          onRetry={() => {
            void corridorQuery.refetch();
          }}
          retrying={corridorQuery.isFetching}
        />
      )}

      <div className="flex items-center gap-1 p-1 bg-slate-950/50 border border-border/20 rounded-xl w-fit">
        <button
          onClick={() => setViewMode("grid")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-[10px] font-bold uppercase tracking-widest transition-all ${
            viewMode === "grid"
              ? "bg-accent text-white glow-accent"
              : "text-muted-foreground hover:text-foreground"
          }`}
        >
          {t("views.grid")}
        </button>
        <button
          onClick={() => setViewMode("table")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-[10px] font-bold uppercase tracking-widest transition-all ${
            viewMode === "table"
              ? "bg-accent text-white glow-accent"
              : "text-muted-foreground hover:text-foreground"
          }`}
        >
          {t("views.table")}
        </button>
        <button
          onClick={() => setViewMode("heatmap")}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-[10px] font-bold uppercase tracking-widest transition-all ${
            viewMode === "heatmap"
              ? "bg-accent text-white glow-accent"
              : "text-muted-foreground hover:text-foreground"
          }`}
        >
          {t("views.heatmap")}
        </button>
      </div>

      {showErrorPanel ? (
        <ObservedDataNotice
          message="Corridor data is unavailable. Retry the request, or check the status page. Routes stay hidden until observed metrics load."
          onRetry={() => {
            void corridorQuery.refetch();
          }}
          retrying={corridorQuery.isFetching}
        />
      ) : loading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {[1, 2, 3, 4, 5, 6].map((i) => (
            <SkeletonCorridorCard key={i} />
          ))}
        </div>
      ) : filteredCorridors.length === 0 ? (
        <div className="py-20 flex flex-col items-center justify-center glass-card rounded-3xl border-dashed">
          <AlertCircle className="w-12 h-12 text-muted-foreground/30 mb-4" />
          <p className="text-sm font-mono text-muted-foreground uppercase tracking-widest">
            {t("empty")}
          </p>
        </div>
      ) : viewMode === "table" ? (
        <VirtualizedCorridorTable
          rows={filteredCorridors}
          sortBy={sortBy}
          sortOrder={sortOrder}
          onSortChange={handleTableSortChange}
          hasNextPage={Boolean(nextCursor)}
          isFetchingNextPage={corridorQuery.isFetching && Boolean(cursor)}
          onLoadMore={handleLoadMore}
          totalCount={totalServerCount}
          lastPatchedId={lastPatchedId}
          isLiveConnected={isLiveConnected}
        />
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
                      {USD_COMPACT.format(corridor.liquidity_depth_usd)}
                    </span>
                  </div>
                  <div className="flex justify-between items-center text-[10px] font-mono uppercase tracking-tighter">
                    <span className="text-muted-foreground">24h Vol</span>
                    <span className="text-foreground font-bold">
                      {USD_COMPACT.format(corridor.liquidity_volume_24h_usd)}
                    </span>
                  </div>
                </div>

                <div className="mt-6 flex items-center justify-between opacity-0 group-hover:opacity-100 transition-opacity">
                  <BookmarkButton
                    id={corridor.id}
                    type="corridor"
                    label={`${corridor.source_asset} → ${corridor.destination_asset}`}
                    href={`/corridors/${corridor.id}`}
                    className="touch-sm"
                  />
                  <ArrowRight className="w-4 h-4 text-accent animate-bounce-x" />
                </div>
              </Link>
            </div>
          ))}
        </div>
      )}

      {viewMode === "grid" && (
        <div className="flex flex-col sm:flex-row items-center justify-between gap-4 glass-card rounded-2xl p-6 border border-border/30">
          <div className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest">
            Telemetry Feed: Viewing {startIndex + 1}-
            {Math.min(endIndex, filteredCorridors.length)} of{" "}
            {filteredCorridors.length} Nodes
            {nextCursor ? " (more pages available)" : ""}
          </div>
          <div className="flex items-center gap-3">
            {nextCursor && (
              <button
                type="button"
                onClick={handleLoadMore}
                disabled={corridorQuery.isFetching}
                className="px-3 py-1.5 rounded-lg border border-accent/30 text-[10px] font-mono uppercase text-accent hover:bg-accent hover:text-white disabled:opacity-50 transition-colors"
              >
                {corridorQuery.isFetching ? "Loading…" : "Load more"}
              </button>
            )}
            <DataTablePagination
              currentPage={currentPage}
              pageSize={pageSize}
              totalItems={filteredCorridors.length}
              onPageChange={onPageChange}
              onPageSizeChange={onPageSizeChange}
            />
          </div>
        </div>
      )}
    </div>
  );
}

export default function CorridorsPage() {
  return (
    <ErrorBoundary>
      <CorridorsPageContent />
    </ErrorBoundary>
  );
}
