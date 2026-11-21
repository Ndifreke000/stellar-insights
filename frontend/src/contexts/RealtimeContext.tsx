"use client";

/**
 * Shared React Context for the tab-wide singleton [`RealtimeClient`] (Issue #2834).
 *
 * Mounts once near the root (inside `ReactQueryProvider`) so every realtime hook
 * (`useWebSocket`, `useRealtimeCorridors`, `useRealtimeAnchors`,
 * `useRealtimeCollaboration`, `NotificationProvider`) multiplexes onto a single
 * WebSocket connection and automatically patches the React Query cache in place
 * on incoming updates.
 */
import React, { createContext, useContext, useEffect, useMemo } from "react";
import {
  QueryClientContext,
  type InfiniteData,
  type QueryClient,
} from "@tanstack/react-query";
import {
  getRealtimeClient,
  normalizeWsUrl,
  type RealtimeClient,
  type WsAnchorUpdate,
  type WsCorridorUpdate,
  type WsMessage,
} from "@/lib/websocket";
import type {
  CorridorDetailData,
  CorridorMetrics,
} from "@/lib/api/corridors";
import type { PaginatedResponse } from "@/lib/api/pagination";
import type { AnchorMetrics } from "@/lib/api/types";
import { queryKeys } from "@/lib/react-query/keys";

export interface RealtimeContextValue {
  client: RealtimeClient;
}

const RealtimeContext = createContext<RealtimeContextValue | null>(null);

// ── Pure React Query cache-patching helpers ──────────────────────────────────
//
// Each helper preserves referential identity of untouched pages and rows so
// memoized virtualized table rows (#2835) only re-render the single patched row.

function applyCorridorPatch(
  row: CorridorMetrics,
  update: WsCorridorUpdate,
): CorridorMetrics {
  const nextSuccessRate = update.success_rate ?? row.success_rate;
  const nextHealthScore = update.health_score ?? row.health_score;
  const nextLiquidity = update.liquidity_depth_usd ?? row.liquidity_depth_usd;
  const nextVolume =
    update.liquidity_volume_24h_usd ??
    update.volume_usd ??
    row.liquidity_volume_24h_usd;
  const nextLatency = update.average_latency_ms ?? row.average_latency_ms;
  const nextAttempts = update.total_transactions ?? row.total_attempts;
  const nextUpdated = update.last_updated ?? row.last_updated;

  if (
    nextSuccessRate === row.success_rate &&
    nextHealthScore === row.health_score &&
    nextLiquidity === row.liquidity_depth_usd &&
    nextVolume === row.liquidity_volume_24h_usd &&
    nextLatency === row.average_latency_ms &&
    nextAttempts === row.total_attempts &&
    nextUpdated === row.last_updated
  ) {
    return row;
  }

  return {
    ...row,
    success_rate: nextSuccessRate,
    health_score: nextHealthScore,
    liquidity_depth_usd: nextLiquidity,
    liquidity_volume_24h_usd: nextVolume,
    average_latency_ms: nextLatency,
    total_attempts: nextAttempts,
    last_updated: nextUpdated,
  };
}

/** Patch a single corridor row inside a `PaginatedResponse<CorridorMetrics>`. */
export function patchCorridorInPaginatedData(
  prev: PaginatedResponse<CorridorMetrics> | undefined,
  update: WsCorridorUpdate,
): PaginatedResponse<CorridorMetrics> | undefined {
  if (!prev || !Array.isArray(prev.data)) return prev;
  const targetKey = update.corridor_key || update.corridor_id;
  if (!targetKey) return prev;

  let changed = false;
  const nextData = prev.data.map((row) => {
    if (row.id !== targetKey) return row;
    const patched = applyCorridorPatch(row, update);
    if (patched !== row) changed = true;
    return patched;
  });

  return changed ? { ...prev, data: nextData } : prev;
}

/** Patch a single corridor row inside an `InfiniteData<PaginatedResponse<CorridorMetrics>>`. */
export function patchCorridorInInfiniteData(
  prev: InfiniteData<PaginatedResponse<CorridorMetrics>> | undefined,
  update: WsCorridorUpdate,
): InfiniteData<PaginatedResponse<CorridorMetrics>> | undefined {
  if (!prev || !Array.isArray(prev.pages)) return prev;

  let changed = false;
  const nextPages = prev.pages.map((page) => {
    const patched = patchCorridorInPaginatedData(page, update);
    if (patched && patched !== page) {
      changed = true;
      return patched;
    }
    return page;
  });

  return changed ? { ...prev, pages: nextPages } : prev;
}

/** Patch an anchor row inside a `PaginatedResponse<AnchorMetrics>` or raw list. */
export function patchAnchorInListData(
  prev: unknown,
  update: WsAnchorUpdate,
): unknown {
  if (!prev || typeof prev !== "object") return prev;

  if ("data" in prev && Array.isArray((prev as { data: unknown[] }).data)) {
    const typed = prev as PaginatedResponse<AnchorMetrics>;
    let changed = false;
    const nextData = typed.data.map((anchor) => {
      if (anchor.id !== update.anchor_id) return anchor;
      changed = true;
      return {
        ...anchor,
        name: update.name ?? anchor.name,
        reliability_score: update.reliability_score ?? anchor.reliability_score,
        status: update.status ?? anchor.status,
      };
    });
    return changed ? { ...typed, data: nextData } : prev;
  }

  if ("anchors" in prev && Array.isArray((prev as { anchors: unknown[] }).anchors)) {
    const typed = prev as { anchors: AnchorMetrics[] };
    let changed = false;
    const nextAnchors = typed.anchors.map((anchor) => {
      if (anchor.id !== update.anchor_id) return anchor;
      changed = true;
      return {
        ...anchor,
        name: update.name ?? anchor.name,
        reliability_score: update.reliability_score ?? anchor.reliability_score,
        status: update.status ?? anchor.status,
      };
    });
    return changed ? { ...typed, anchors: nextAnchors } : prev;
  }

  return prev;
}

/**
 * Apply an incoming WebSocket event directly to the React Query cache.
 */
export function syncRealtimeMessageToQueryCache(
  queryClient: QueryClient,
  message: WsMessage,
): void {
  if (message.type === "corridor_update") {
    const update = message as unknown as WsCorridorUpdate;
    const corridorKey = update.corridor_key || update.corridor_id;
    if (!corridorKey) return;

    // 1. Patch detail cache in place
    queryClient.setQueryData<CorridorDetailData>(
      queryKeys.corridorDetail(corridorKey),
      (prev) => {
        if (!prev) return prev;
        const patched = applyCorridorPatch(prev.corridor, update);
        return patched === prev.corridor ? prev : { ...prev, corridor: patched };
      },
    );

    // 2. Patch matching row across every cached corridor list (both paginated & infinite)
    queryClient.setQueriesData<unknown>(
      { queryKey: queryKeys.corridorLists },
      (prev: unknown) => {
        if (!prev || typeof prev !== "object") return prev;
        if ("pages" in prev) {
          return patchCorridorInInfiniteData(
            prev as InfiniteData<PaginatedResponse<CorridorMetrics>>,
            update,
          );
        }
        return patchCorridorInPaginatedData(
          prev as PaginatedResponse<CorridorMetrics>,
          update,
        );
      },
    );
    return;
  }

  if (message.type === "anchor_update") {
    const update = message as unknown as WsAnchorUpdate;
    if (!update.anchor_id) return;

    queryClient.setQueriesData<unknown>(
      { queryKey: queryKeys.anchorLists },
      (prev: unknown) => patchAnchorInListData(prev, update),
    );
    queryClient.setQueryData<Record<string, unknown>>(
      queryKeys.anchorDetail(update.anchor_id),
      (prev) => {
        if (!prev) return prev;
        const anchor = prev.anchor as Record<string, unknown> | undefined;
        if (!anchor) return prev;
        return {
          ...prev,
          anchor: {
            ...anchor,
            name: update.name ?? anchor.name,
            reliability_score:
              update.reliability_score ?? anchor.reliability_score,
            status: update.status ?? anchor.status,
          },
        };
      },
    );
    return;
  }

  if (message.type === "snapshot_update") {
    void queryClient.invalidateQueries({ queryKey: queryKeys.corridorLists });
    void queryClient.invalidateQueries({ queryKey: queryKeys.anchorLists });
  }
}

// ── Provider & hooks ──────────────────────────────────────────────────────────

export interface RealtimeProviderProps {
  children: React.ReactNode;
  /** Optional explicit WebSocket URL override (defaults to canonical backend `/ws`). */
  url?: string;
  /** Optional pre-constructed client (useful in tests). */
  client?: RealtimeClient;
  /** Automatically patch React Query cache on realtime updates (default: true). */
  autoSyncCache?: boolean;
}

export function RealtimeProvider({
  children,
  url,
  client: injectedClient,
  autoSyncCache = true,
}: RealtimeProviderProps) {
  // Optional QueryClient lookup so tests rendering RealtimeProvider standalone still work.
  const queryClient = useContext(QueryClientContext);

  const client = useMemo(
    () => injectedClient ?? getRealtimeClient(url ? { url } : undefined),
    [injectedClient, url],
  );

  // Sync incoming realtime updates into React Query cache once for the entire tab.
  useEffect(() => {
    if (!autoSyncCache || !queryClient) return;
    return client.onAny((message) => {
      syncRealtimeMessageToQueryCache(queryClient, message);
    });
  }, [client, queryClient, autoSyncCache]);

  const value = useMemo<RealtimeContextValue>(() => ({ client }), [client]);

  return (
    <RealtimeContext.Provider value={value}>
      {children}
    </RealtimeContext.Provider>
  );
}

/**
 * Retrieve the shared [`RealtimeClient`] from context when available (and matching
 * `url`), or fall back to the module-level tab singleton `getRealtimeClient`.
 */
export function useRealtimeClient(url?: string): RealtimeClient {
  const ctx = useContext(RealtimeContext);
  const normalized = url ? normalizeWsUrl(url) : undefined;

  return useMemo(() => {
    if (ctx && (!normalized || normalized === ctx.client.getUrl())) {
      return ctx.client;
    }
    return getRealtimeClient(normalized ? { url: normalized } : undefined);
  }, [ctx, normalized]);
}
