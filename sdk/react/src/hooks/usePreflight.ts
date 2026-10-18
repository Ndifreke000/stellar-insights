import { useCallback, useEffect, useRef, useSyncExternalStore } from "react";
import type { PayRaider } from "@payraider/sdk";
import { usePayRaiderClient } from "../PayRaiderProvider.js";

/** Request body for `client.preflight.check` (source/destination asset, amount, threshold). */
export type PreflightParams = Parameters<PayRaider["preflight"]["check"]>[0];
/** Response of `client.preflight.check`: `decision`, `checks`, `alternatives`, ... */
export type PreflightResult = Awaited<ReturnType<PayRaider["preflight"]["check"]>>;

export interface UsePreflightOptions {
  /**
   * How long (ms) a cached result counts as fresh. A fresh result is served
   * without a request; a stale one is served immediately *and* revalidated in
   * the background. Default 30 000.
   */
  staleTime?: number;
  /** Set `false` to skip fetching (cached data is still returned). Default `true`. */
  enabled?: boolean;
}

export interface UsePreflightResult {
  /** Latest result; kept while revalidating and after a failed revalidation. */
  data: PreflightResult | null;
  /** Error from the most recent request, cleared on the next success. */
  error: Error | null;
  /** `true` until the first result (or error) arrives for these params. */
  isLoading: boolean;
  /** `true` whenever a request for these params is in flight. */
  isValidating: boolean;
  /** Force a revalidation, ignoring `staleTime`. Never rejects. */
  refetch: () => Promise<void>;
}

export const DEFAULT_PREFLIGHT_STALE_TIME_MS = 30_000;
/** Cap on cached param combinations per client; idle entries are evicted first. */
const MAX_ENTRIES = 100;

interface CacheState {
  data: PreflightResult | null;
  error: Error | null;
  /** `Date.now()` of the last successful fetch; 0 = never fetched. */
  updatedAt: number;
  isValidating: boolean;
}

interface CacheEntry {
  state: CacheState;
  inflight: Promise<PreflightResult> | null;
  listeners: Set<() => void>;
}

const EMPTY_STATE: CacheState = { data: null, error: null, updatedAt: 0, isValidating: false };

// Scoped per client so two providers (e.g. mainnet + testnet) never share results.
let caches = new WeakMap<PayRaider, Map<string, CacheEntry>>();

/**
 * Stable cache key: same params in any key order map to the same entry, and
 * `undefined` fields are ignored (matching what is sent over the wire).
 */
export function preflightKey(params: PreflightParams): string {
  const entries = Object.entries(params as unknown as Record<string, unknown>)
    .filter(([, v]) => v !== undefined)
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
  return JSON.stringify(entries);
}

function entryFor(client: PayRaider, key: string): CacheEntry {
  let cache = caches.get(client);
  if (!cache) {
    cache = new Map();
    caches.set(client, cache);
  }
  let entry = cache.get(key);
  if (!entry) {
    if (cache.size >= MAX_ENTRIES) {
      for (const [k, e] of cache) {
        if (e.listeners.size === 0 && !e.inflight) {
          cache.delete(k);
          break;
        }
      }
    }
    entry = { state: EMPTY_STATE, inflight: null, listeners: new Set() };
    cache.set(key, entry);
  }
  return entry;
}

function update(entry: CacheEntry, patch: Partial<CacheState>): void {
  entry.state = { ...entry.state, ...patch };
  entry.listeners.forEach((listener) => listener());
}

/** Fetch once per key at a time: concurrent callers share the in-flight request. */
function revalidate(
  client: PayRaider,
  key: string,
  params: PreflightParams,
): Promise<PreflightResult> {
  const entry = entryFor(client, key);
  if (entry.inflight) return entry.inflight;

  const request = client.preflight.check(params).then(
    (data) => {
      update(entry, { data, error: null, updatedAt: Date.now(), isValidating: false });
      return data;
    },
    (err: unknown) => {
      const error = err instanceof Error ? err : new Error(String(err));
      update(entry, { error, isValidating: false });
      throw error;
    },
  );
  entry.inflight = request;
  const clear = () => {
    if (entry.inflight === request) entry.inflight = null;
  };
  request.then(clear, clear);
  update(entry, { isValidating: true });
  return request;
}

/**
 * Drop cached preflight results — for one client, or all clients when called
 * without arguments (e.g. on sign-out or between tests).
 */
export function clearPreflightCache(client?: PayRaider): void {
  if (client) caches.delete(client);
  else caches = new WeakMap();
}

const noop = () => {};

/**
 * Pre-payment check for a corridor, with request dedupe and
 * stale-while-revalidate caching.
 *
 * ```tsx
 * const { data, isLoading } = usePreflight({
 *   source_asset: "USDC",
 *   destination_asset: "NGN",
 *   amount_usd: 2500,
 * });
 * if (data?.decision === "proceed") pay();
 * ```
 *
 * Pass `null` to skip the request until the inputs are ready. Every component
 * that asks for the same params shares a single request and a single cached
 * result. A stale result is returned at once while a fresh one loads.
 */
export function usePreflight(
  params: PreflightParams | null | undefined,
  options: UsePreflightOptions = {},
): UsePreflightResult {
  const client = usePayRaiderClient();
  const { staleTime = DEFAULT_PREFLIGHT_STALE_TIME_MS, enabled = true } = options;
  const key = params ? preflightKey(params) : null;
  const active = enabled && key !== null;

  // Callers usually pass a fresh object literal each render; `key` tracks the
  // content, the ref gives effects the latest value without re-running them.
  const paramsRef = useRef(params);
  paramsRef.current = params;

  const subscribe = useCallback(
    (onChange: () => void) => {
      if (key === null) return noop;
      const entry = entryFor(client, key);
      entry.listeners.add(onChange);
      return () => {
        entry.listeners.delete(onChange);
      };
    },
    [client, key],
  );
  const getSnapshot = useCallback(
    () => (key === null ? EMPTY_STATE : entryFor(client, key).state),
    [client, key],
  );
  const state = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

  useEffect(() => {
    const current = paramsRef.current;
    if (!active || key === null || !current) return;
    const { updatedAt } = entryFor(client, key).state;
    const isFresh = updatedAt > 0 && Date.now() - updatedAt < staleTime;
    if (!isFresh) revalidate(client, key, current).catch(noop);
  }, [client, key, active, staleTime]);

  const refetch = useCallback(async () => {
    const current = paramsRef.current;
    if (key === null || !current) return;
    await revalidate(client, key, current).catch(noop);
  }, [client, key]);

  return {
    data: state.data,
    error: state.error,
    isLoading: active && state.data === null && state.error === null,
    isValidating: state.isValidating,
    refetch,
  };
}
