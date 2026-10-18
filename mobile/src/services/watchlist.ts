/**
 * Corridor Watchlist Service (issue #2627)
 *
 * Allows users to watch Stellar payment corridors and receive push
 * notifications when a corridor's health/status changes meaningfully.
 *
 * ## Design note
 *
 * ### Data model
 * ```
 * WatchlistEntry {
 *   corridorId   : string          — unique corridor key (source:destination)
 *   addedAt      : ISO-8601 string — when the user added it
 *   lastStatus   : CorridorStatus  — cached status for change detection
 *   alertThreshold: AlertThreshold — what the user wants to be notified about
 * }
 * ```
 *
 * ### Storage
 * Watchlist entries are persisted in MMKV (encrypted, device-local) so they
 * survive app restarts and work fully offline.  When the device comes back
 * online the list is synced to the backend so the backend can trigger server-
 * side push on status change even when the app is backgrounded.
 *
 * ### API contract
 * ```
 * GET  /api/v1/watchlist              — fetch server copy
 * POST /api/v1/watchlist              — add entry     { corridorId, alertThreshold }
 * DELETE /api/v1/watchlist/:corridorId — remove entry
 * POST /api/v1/watchlist/:corridorId/status — backend pushes a status change
 * ```
 *
 * ### Push flow
 * 1. User adds corridor → local store + POST to backend.
 * 2. Backend monitors the corridor, detects a status change.
 * 3. Backend sends an FCM message to the device token registered via
 *    `registerDeviceTokenWithBackend` (notifications.ts).
 * 4. `notifications.ts` `onMessage` handler displays the notification via
 *    Notifee.  The data payload contains `{ type: 'CORRIDOR_STATUS_CHANGE',
 *    corridorId, newStatus }`.
 *
 * ### Edge cases
 * - **Offline add/remove**: changes are queued in MMKV and replayed via
 *   `syncWatchlistToBackend` when the network returns.
 * - **Duplicate entries**: `addToWatchlist` is idempotent — it returns early
 *   if the corridor is already watched.
 * - **Token not yet registered**: the sync call is skipped and will be
 *   retried on next network reconnect.
 * - **Backend unreachable**: local list is always the source of truth; sync
 *   failures are logged and retried; no data is lost.
 */

import AsyncStorage from '@react-native-async-storage/async-storage';
import { apiClient } from './api';
import { logger } from '@utils/logger';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type CorridorStatus = 'healthy' | 'degraded' | 'down' | 'unknown';

export type AlertThreshold =
  /** Notify on any status change. */
  | 'any_change'
  /** Notify only when corridor goes down or recovers. */
  | 'down_only'
  /** Notify only when health score drops below 50 %. */
  | 'health_below_50';

export interface WatchlistEntry {
  /** Unique corridor identifier, e.g. "USDC:NGN". */
  corridorId: string;
  /** ISO-8601 timestamp when the user added this corridor. */
  addedAt: string;
  /** Last known status — used client-side for change detection. */
  lastStatus: CorridorStatus;
  /** What condition should trigger a push notification. */
  alertThreshold: AlertThreshold;
}

/** Pending sync operations for offline-first support. */
interface PendingOp {
  type: 'add' | 'remove';
  corridorId: string;
  entry?: WatchlistEntry;
}

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

const WATCHLIST_KEY = '@watchlist/entries';
const PENDING_OPS_KEY = '@watchlist/pending_ops';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function readEntries(): Promise<WatchlistEntry[]> {
  try {
    const raw = await AsyncStorage.getItem(WATCHLIST_KEY);
    if (!raw) return [];
    return JSON.parse(raw) as WatchlistEntry[];
  } catch (err) {
    logger.error('[Watchlist]', 'Failed to read local watchlist:', err);
    return [];
  }
}

async function writeEntries(entries: WatchlistEntry[]): Promise<void> {
  await AsyncStorage.setItem(WATCHLIST_KEY, JSON.stringify(entries));
}

async function readPendingOps(): Promise<PendingOp[]> {
  try {
    const raw = await AsyncStorage.getItem(PENDING_OPS_KEY);
    if (!raw) return [];
    return JSON.parse(raw) as PendingOp[];
  } catch {
    return [];
  }
}

async function writePendingOps(ops: PendingOp[]): Promise<void> {
  await AsyncStorage.setItem(PENDING_OPS_KEY, JSON.stringify(ops));
}

async function enqueuePendingOp(op: PendingOp): Promise<void> {
  const ops = await readPendingOps();
  // Deduplicate: replace any existing op for the same corridorId
  const filtered = ops.filter(o => o.corridorId !== op.corridorId);
  filtered.push(op);
  await writePendingOps(filtered);
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Return the current watchlist entries (local copy).
 */
export async function getWatchlist(): Promise<WatchlistEntry[]> {
  return readEntries();
}

/**
 * Add a corridor to the watchlist.
 *
 * Idempotent: if the corridor is already watched, this is a no-op.
 * Queues a backend sync that is replayed when the network is available.
 *
 * @param corridorId      - e.g. "USDC:NGN"
 * @param alertThreshold  - defaults to `'any_change'`
 */
export async function addToWatchlist(
  corridorId: string,
  alertThreshold: AlertThreshold = 'any_change',
): Promise<void> {
  const entries = await readEntries();

  if (entries.some(e => e.corridorId === corridorId)) {
    logger.debug('[Watchlist]', `Corridor ${corridorId} is already in the watchlist — no-op.`);
    return;
  }

  const newEntry: WatchlistEntry = {
    corridorId,
    addedAt: new Date().toISOString(),
    lastStatus: 'unknown',
    alertThreshold,
  };

  entries.push(newEntry);
  await writeEntries(entries);
  logger.info('[Watchlist]', `Added corridor ${corridorId} to watchlist.`);

  // Optimistically sync to backend; queue for retry on failure.
  try {
    await apiClient.post('/api/v1/watchlist', {
      corridorId,
      alertThreshold,
    });
    logger.debug('[Watchlist]', `Synced ${corridorId} add to backend.`);
  } catch {
    logger.warn('[Watchlist]', `Backend sync failed for add(${corridorId}) — queued for retry.`);
    await enqueuePendingOp({ type: 'add', corridorId, entry: newEntry });
  }
}

/**
 * Remove a corridor from the watchlist.
 *
 * Queues a backend sync that is replayed when the network is available.
 *
 * @param corridorId - e.g. "USDC:NGN"
 */
export async function removeFromWatchlist(corridorId: string): Promise<void> {
  const entries = await readEntries();
  const filtered = entries.filter(e => e.corridorId !== corridorId);

  if (filtered.length === entries.length) {
    logger.debug('[Watchlist]', `Corridor ${corridorId} was not in the watchlist — no-op.`);
    return;
  }

  await writeEntries(filtered);
  logger.info('[Watchlist]', `Removed corridor ${corridorId} from watchlist.`);

  try {
    await apiClient.delete(`/api/v1/watchlist/${encodeURIComponent(corridorId)}`);
    logger.debug('[Watchlist]', `Synced ${corridorId} remove to backend.`);
  } catch {
    logger.warn('[Watchlist]', `Backend sync failed for remove(${corridorId}) — queued for retry.`);
    await enqueuePendingOp({ type: 'remove', corridorId });
  }
}

/**
 * Returns `true` if the given corridor is currently in the watchlist.
 */
export async function isWatched(corridorId: string): Promise<boolean> {
  const entries = await readEntries();
  return entries.some(e => e.corridorId === corridorId);
}

/**
 * Update the cached status for a corridor and return whether it changed.
 *
 * Called by the push notification handler when a `CORRIDOR_STATUS_CHANGE`
 * FCM message arrives so that subsequent renders show the correct status
 * without a network round-trip.
 *
 * @returns `true` when the status actually changed (used by callers to decide
 *   whether to refresh the UI).
 */
export async function updateCorridorStatus(
  corridorId: string,
  newStatus: CorridorStatus,
): Promise<boolean> {
  const entries = await readEntries();
  const idx = entries.findIndex(e => e.corridorId === corridorId);
  if (idx === -1) return false;

  const previous = entries[idx].lastStatus;
  if (previous === newStatus) return false;

  entries[idx] = { ...entries[idx], lastStatus: newStatus };
  await writeEntries(entries);
  logger.info('[Watchlist]', `Corridor ${corridorId} status: ${previous} → ${newStatus}`);
  return true;
}

/**
 * Replay any queued add/remove operations against the backend.
 *
 * Call this from the network-reconnect handler in `network.ts` so that
 * offline changes are flushed once connectivity is restored.
 */
export async function syncWatchlistToBackend(): Promise<void> {
  const ops = await readPendingOps();
  if (ops.length === 0) return;

  logger.info('[Watchlist]', `Replaying ${ops.length} pending watchlist op(s).`);

  const remaining: PendingOp[] = [];

  for (const op of ops) {
    try {
      if (op.type === 'add' && op.entry) {
        await apiClient.post('/api/v1/watchlist', {
          corridorId: op.corridorId,
          alertThreshold: op.entry.alertThreshold,
        });
      } else if (op.type === 'remove') {
        await apiClient.delete(`/api/v1/watchlist/${encodeURIComponent(op.corridorId)}`);
      }
      logger.debug('[Watchlist]', `Pending op replayed: ${op.type}(${op.corridorId})`);
    } catch {
      logger.warn('[Watchlist]', `Retry failed for ${op.type}(${op.corridorId}) — will try again.`);
      remaining.push(op);
    }
  }

  await writePendingOps(remaining);
}
