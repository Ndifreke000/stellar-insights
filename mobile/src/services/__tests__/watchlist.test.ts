/**
 * Tests for the corridor watchlist service (issue #2627).
 *
 * Main path + edge cases:
 *  1. Add a corridor — stored locally and synced to backend.
 *  2. Add duplicate — idempotent, no duplicate entry, no extra API call.
 *  3. Remove a corridor — removed locally and synced.
 *  4. Offline add — entry stored locally, pending op queued; syncWatchlistToBackend replays it.
 *  5. Status update — correctly detects change, no-op on same status.
 */

import AsyncStorage from '@react-native-async-storage/async-storage';

// Mock the API client
const mockPost = jest.fn();
const mockDelete = jest.fn();
jest.mock('@services/api', () => ({
  apiClient: {
    post: (...args: unknown[]) => mockPost(...args),
    delete: (...args: unknown[]) => mockDelete(...args),
  },
}));

// Mock logger to silence output
jest.mock('@utils/logger', () => ({
  logger: { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() },
}));

import {
  addToWatchlist,
  removeFromWatchlist,
  isWatched,
  getWatchlist,
  updateCorridorStatus,
  syncWatchlistToBackend,
} from '@services/watchlist';

const PENDING_OPS_KEY = '@watchlist/pending_ops';

beforeEach(async () => {
  await AsyncStorage.clear();
  mockPost.mockReset();
  mockDelete.mockReset();
});

// ---------------------------------------------------------------------------
// 1. Add a corridor
// ---------------------------------------------------------------------------
describe('addToWatchlist', () => {
  it('stores the entry locally', async () => {
    mockPost.mockResolvedValue({});
    await addToWatchlist('USDC:NGN');

    const entries = await getWatchlist();
    expect(entries).toHaveLength(1);
    expect(entries[0].corridorId).toBe('USDC:NGN');
    expect(entries[0].alertThreshold).toBe('any_change');
    expect(entries[0].lastStatus).toBe('unknown');
  });

  it('calls the backend with corridorId and alertThreshold', async () => {
    mockPost.mockResolvedValue({});
    await addToWatchlist('USDC:KES', 'down_only');

    expect(mockPost).toHaveBeenCalledWith('/api/v1/watchlist', {
      corridorId: 'USDC:KES',
      alertThreshold: 'down_only',
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Duplicate is idempotent
  // ---------------------------------------------------------------------------
  it('is idempotent — does not add duplicate entries', async () => {
    mockPost.mockResolvedValue({});
    await addToWatchlist('USDC:NGN');
    await addToWatchlist('USDC:NGN'); // second call

    const entries = await getWatchlist();
    expect(entries).toHaveLength(1);
    // Backend should only have been called once
    expect(mockPost).toHaveBeenCalledTimes(1);
  });
});

// ---------------------------------------------------------------------------
// 3. Remove a corridor
// ---------------------------------------------------------------------------
describe('removeFromWatchlist', () => {
  it('removes the entry locally and calls backend DELETE', async () => {
    mockPost.mockResolvedValue({});
    mockDelete.mockResolvedValue({});

    await addToWatchlist('USDC:NGN');
    await removeFromWatchlist('USDC:NGN');

    expect(await isWatched('USDC:NGN')).toBe(false);
    expect(mockDelete).toHaveBeenCalledWith('/api/v1/watchlist/USDC%3ANGN');
  });

  it('is a no-op when the corridor is not in the watchlist', async () => {
    mockDelete.mockResolvedValue({});
    await removeFromWatchlist('USDC:ZAR'); // never added

    expect(mockDelete).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// 4. Offline add queues a pending op
// ---------------------------------------------------------------------------
describe('offline behaviour', () => {
  it('queues a pending op when backend is unreachable on add', async () => {
    mockPost.mockRejectedValue(new Error('Network error'));

    await addToWatchlist('USDC:NGN');

    // Entry still exists locally
    expect(await isWatched('USDC:NGN')).toBe(true);

    // Pending op was written
    const rawOps = await AsyncStorage.getItem(PENDING_OPS_KEY);
    const ops = JSON.parse(rawOps ?? '[]');
    expect(ops).toHaveLength(1);
    expect(ops[0].type).toBe('add');
    expect(ops[0].corridorId).toBe('USDC:NGN');
  });

  it('syncWatchlistToBackend replays pending ops', async () => {
    mockPost.mockRejectedValueOnce(new Error('offline')).mockResolvedValue({});

    await addToWatchlist('USDC:NGN'); // fails → queued

    mockPost.mockClear();
    await syncWatchlistToBackend(); // replay

    expect(mockPost).toHaveBeenCalledWith('/api/v1/watchlist', expect.objectContaining({
      corridorId: 'USDC:NGN',
    }));

    // Pending ops cleared after successful replay
    const rawOps = await AsyncStorage.getItem(PENDING_OPS_KEY);
    expect(JSON.parse(rawOps ?? '[]')).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// 5. Status update
// ---------------------------------------------------------------------------
describe('updateCorridorStatus', () => {
  it('returns true and persists the new status when the status changes', async () => {
    mockPost.mockResolvedValue({});
    await addToWatchlist('USDC:NGN');

    const changed = await updateCorridorStatus('USDC:NGN', 'degraded');
    expect(changed).toBe(true);

    const entries = await getWatchlist();
    expect(entries[0].lastStatus).toBe('degraded');
  });

  it('returns false when the new status is the same as current', async () => {
    mockPost.mockResolvedValue({});
    await addToWatchlist('USDC:NGN');
    await updateCorridorStatus('USDC:NGN', 'degraded');

    const changed = await updateCorridorStatus('USDC:NGN', 'degraded');
    expect(changed).toBe(false);
  });

  it('returns false for an unrecognised corridor', async () => {
    const changed = await updateCorridorStatus('USDC:UNKNOWN', 'down');
    expect(changed).toBe(false);
  });
});
