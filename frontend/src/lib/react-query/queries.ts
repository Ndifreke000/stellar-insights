"use client";

/**
 * Server-state hooks — the ONLY way components should read API data.
 *
 * Every hook here shares the app-wide QueryClient, so two components asking for
 * the same resource share one request and one cache entry. Keys come from
 * `./keys`; realtime updates are applied through `useRealtimeCacheSync`.
 *
 * See docs/FRONTEND_STATE_MANAGEMENT.md.
 */
import { useCallback } from "react";
import {
  keepPreviousData,
  useQueries,
  useQuery,
  useQueryClient,
  type UseQueryResult,
} from "@tanstack/react-query";

import {
  getCorridorDetail,
  getCorridors,
  type CorridorDetailData,
  type CorridorFilters,
} from "@/lib/api/corridors";
import {
  fetchAnchors,
  getAnchorDetail,
  type ListAnchorsParams,
} from "@/lib/api/anchor";
import { getProposals } from "@/lib/governance-api";
import type { ProposalStatus } from "@/types/governance";
import type { CorridorUpdate } from "@/hooks/useRealtimeCorridors";
import { queryKeys } from "./keys";

/** Metrics refresh server-side every few minutes; don't refetch more often. */
const METRICS_STALE_TIME = 60 * 1000;

// ── Corridors ────────────────────────────────────────────────────────────────

export function useCorridors(filters: CorridorFilters = {}) {
  return useQuery({
    queryKey: queryKeys.corridorList(filters),
    queryFn: () => getCorridors(filters),
    staleTime: METRICS_STALE_TIME,
    // Keep showing the previous result while a new filter combination loads.
    placeholderData: keepPreviousData,
  });
}

export function useCorridorDetail(corridorId: string | undefined) {
  return useQuery({
    queryKey: queryKeys.corridorDetail(corridorId ?? ""),
    queryFn: () => getCorridorDetail(corridorId as string),
    enabled: !!corridorId,
    staleTime: METRICS_STALE_TIME,
  });
}

// Module-level so its identity is stable: React Query then only recomputes (and
// returns a new object) when an underlying query result actually changes.
function combineCorridorDetails(results: UseQueryResult<CorridorDetailData>[]) {
  return {
    data: results.map((r) => r.data),
    isPending: results.some((r) => r.isPending && !r.isError),
  };
}

/**
 * Fetch several corridor details in parallel, sharing cache with
 * `useCorridorDetail`. `data[i]` is `undefined` until (or if never) loaded.
 */
export function useCorridorDetails(corridorIds: string[]) {
  return useQueries({
    queries: corridorIds.map((id) => ({
      queryKey: queryKeys.corridorDetail(id),
      queryFn: () => getCorridorDetail(id),
      staleTime: METRICS_STALE_TIME,
    })),
    combine: combineCorridorDetails,
  });
}

// ── Anchors ──────────────────────────────────────────────────────────────────

export function useAnchors(params: ListAnchorsParams = {}) {
  return useQuery({
    queryKey: queryKeys.anchorList(params),
    queryFn: () => fetchAnchors(params),
    staleTime: METRICS_STALE_TIME,
    placeholderData: keepPreviousData,
  });
}

export function useAnchorDetail(address: string | undefined) {
  return useQuery({
    queryKey: queryKeys.anchorDetail(address ?? ""),
    queryFn: () => getAnchorDetail(address as string),
    enabled: !!address,
    staleTime: METRICS_STALE_TIME,
  });
}

// ── Governance ───────────────────────────────────────────────────────────────

export function useProposals(status?: ProposalStatus) {
  return useQuery({
    queryKey: queryKeys.proposals(status),
    queryFn: () => getProposals(status),
    staleTime: 30 * 1000,
    placeholderData: keepPreviousData,
  });
}

// ── Realtime → cache synchronisation ─────────────────────────────────────────

/**
 * Callbacks that apply WebSocket events to the query cache, so every component
 * showing that resource updates together. Pass them to the realtime hooks:
 *
 *   const sync = useRealtimeCacheSync();
 *   useRealtimeCorridors({ onCorridorUpdate: sync.onCorridorUpdate });
 */
export function useRealtimeCacheSync() {
  const queryClient = useQueryClient();

  const onCorridorUpdate = useCallback(
    (update: CorridorUpdate) => {
      // Patch the detail view in place — the event carries the new numbers.
      queryClient.setQueryData<CorridorDetailData>(
        queryKeys.corridorDetail(update.corridor_key),
        (prev) =>
          prev && {
            ...prev,
            corridor: {
              ...prev.corridor,
              success_rate: update.success_rate ?? prev.corridor.success_rate,
              health_score: update.health_score ?? prev.corridor.health_score,
              last_updated: update.last_updated ?? prev.corridor.last_updated,
            },
          },
      );
      // Lists are filtered/sorted server-side, so just mark them stale; active
      // lists refetch in the background, inactive ones on next use.
      void queryClient.invalidateQueries({ queryKey: queryKeys.corridorLists });
    },
    [queryClient],
  );

  const onAnchorUpdate = useCallback(
    (update: { anchor_id: string }) => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.anchorLists });
      void queryClient.invalidateQueries({
        queryKey: queryKeys.anchorDetail(update.anchor_id),
      });
    },
    [queryClient],
  );

  return { onCorridorUpdate, onAnchorUpdate };
}
