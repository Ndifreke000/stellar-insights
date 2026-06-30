/**
 * Offline-first local data layer.
 *
 * Persistence is handled by AsyncStorage.  Each entity type gets its own
 * namespace so keys never collide.  All operations are async and swallow
 * individual errors gracefully — a cache miss is never fatal.
 */

import AsyncStorage from '@react-native-async-storage/async-storage';
import { CACHE_KEYS } from '@config/constants';
import { ANCHOR_DETAIL_CACHE_PREFIX } from '@types/anchor';
import { CORRIDOR_DETAIL_CACHE_PREFIX } from '@types/corridor';

// ─── Schema version ──────────────────────────────────────────────────────────

const DB_VERSION_KEY = '@db_version';
const CURRENT_DB_VERSION = 1;

// ─── Asset types ─────────────────────────────────────────────────────────────

export interface AssetRecord {
  code: string;
  issuer: string;
  domain?: string;
  verified: boolean;
  /** ISO-8601 timestamp of when this record was last fetched from the API */
  cachedAt: string;
}

export interface AssetsCache {
  assets: AssetRecord[];
  total: number;
  cachedAt: string;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async function safeGet<T>(key: string): Promise<T | null> {
  try {
    const raw = await AsyncStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : null;
  } catch {
    return null;
  }
}

async function safeSet(key: string, value: unknown): Promise<void> {
  try {
    await AsyncStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Cache writes are best-effort; offline reads will fall back to mock data.
  }
}

async function safeRemove(key: string): Promise<void> {
  try {
    await AsyncStorage.removeItem(key);
  } catch {
    // Ignore removal errors — stale keys are harmless.
  }
}

// ─── Schema migration ────────────────────────────────────────────────────────

async function runMigrations(): Promise<void> {
  const stored = await safeGet<number>(DB_VERSION_KEY);
  const version = stored ?? 0;

  if (version < 1) {
    // v1: initial schema — nothing to migrate from a previous version.
    await safeSet(DB_VERSION_KEY, 1);
  }
}

// ─── Initialisation ──────────────────────────────────────────────────────────

/**
 * Initialises the local data layer.
 *
 * Call this once at app startup (already wired into `initializeApp`).
 * Runs any pending schema migrations before returning.
 */
export async function initializeDatabase(): Promise<void> {
  await runMigrations();
}

// ─── Clear ───────────────────────────────────────────────────────────────────

/**
 * Removes all cached entity data from AsyncStorage.
 *
 * Sync-queue items (stored in MMKV via `useOfflineQueue`) are intentionally
 * left untouched so pending mutations survive a cache reset.
 */
export async function clearDatabase(): Promise<void> {
  const allKeys = await AsyncStorage.getAllKeys();

  // Collect keys that belong to the offline cache namespaces.
  const cacheKeys = allKeys.filter(
    k =>
      k === CACHE_KEYS.CORRIDORS ||
      k === CACHE_KEYS.ANCHORS ||
      k === CACHE_KEYS.ASSETS ||
      k === CACHE_KEYS.ANALYTICS ||
      k.startsWith(CORRIDOR_DETAIL_CACHE_PREFIX) ||
      k.startsWith(ANCHOR_DETAIL_CACHE_PREFIX),
  );

  if (cacheKeys.length > 0) {
    await AsyncStorage.multiRemove(cacheKeys);
  }
}

// ─── Assets ──────────────────────────────────────────────────────────────────

/** Reads the cached assets list, returning `null` if absent or unreadable. */
export async function getCachedAssets(): Promise<AssetsCache | null> {
  return safeGet<AssetsCache>(CACHE_KEYS.ASSETS);
}

/** Persists the assets list to the local cache. */
export async function setCachedAssets(data: AssetsCache): Promise<void> {
  await safeSet(CACHE_KEYS.ASSETS, data);
}

/** Removes the assets cache. */
export async function clearCachedAssets(): Promise<void> {
  await safeRemove(CACHE_KEYS.ASSETS);
}

// ─── Corridors list ──────────────────────────────────────────────────────────

export interface CorridorsCache {
  corridors: unknown[];
  total: number;
  cachedAt: string;
}

/** Reads the cached corridors list. */
export async function getCachedCorridorsList(): Promise<CorridorsCache | null> {
  return safeGet<CorridorsCache>(CACHE_KEYS.CORRIDORS);
}

/** Persists the corridors list. */
export async function setCachedCorridorsList(data: CorridorsCache): Promise<void> {
  await safeSet(CACHE_KEYS.CORRIDORS, data);
}

// ─── Anchors list ────────────────────────────────────────────────────────────

export interface AnchorsCache {
  anchors: unknown[];
  total: number;
  cachedAt: string;
}

/** Reads the cached anchors list. */
export async function getCachedAnchorsList(): Promise<AnchorsCache | null> {
  return safeGet<AnchorsCache>(CACHE_KEYS.ANCHORS);
}

/** Persists the anchors list. */
export async function setCachedAnchorsList(data: AnchorsCache): Promise<void> {
  await safeSet(CACHE_KEYS.ANCHORS, data);
}
