/**
 * Functional tests for the useDataRefresh hook.
 *
 * Uses Vitest's fake timer APIs so the tests run without wall-clock delays.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useDataRefresh } from '@/hooks/useDataRefresh';

// ── helpers ──────────────────────────────────────────────────────────────────

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.clearAllMocks();
});

// ── initial state ─────────────────────────────────────────────────────────────

describe('useDataRefresh – initial state', () => {
  it('starts with lastUpdated = null', () => {
    const { result } = renderHook(() => useDataRefresh());
    expect(result.current.lastUpdated).toBeNull();
  });

  it('starts with isRefreshing = false', () => {
    const { result } = renderHook(() => useDataRefresh());
    expect(result.current.isRefreshing).toBe(false);
  });

  it('sets secondsUntilRefresh to the configured interval on mount', () => {
    const { result } = renderHook(() =>
      useDataRefresh({ refreshIntervalMs: 60_000 }),
    );
    expect(result.current.secondsUntilRefresh).toBe(60);
  });

  it('defaults to a 30-second countdown', () => {
    const { result } = renderHook(() => useDataRefresh());
    expect(result.current.secondsUntilRefresh).toBe(30);
  });
});

// ── countdown ────────────────────────────────────────────────────────────────

describe('useDataRefresh – countdown', () => {
  it('decrements secondsUntilRefresh every second', async () => {
    const { result } = renderHook(() =>
      useDataRefresh({ refreshIntervalMs: 10_000 }),
    );

    expect(result.current.secondsUntilRefresh).toBe(10);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(3_000); // 3 seconds
    });

    expect(result.current.secondsUntilRefresh).toBe(7);
  });

  it('reaches 0 when the full interval elapses', async () => {
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() =>
      useDataRefresh({ refreshIntervalMs: 5_000, onRefresh }),
    );

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });

    // The hook calls startCountdown() immediately after the auto-refresh
    // which resets the counter to refreshIntervalSec. We verify the interval
    // elapsed by confirming onRefresh was invoked and the counter was reset.
    expect(onRefresh).toHaveBeenCalledTimes(1);
    expect(result.current.secondsUntilRefresh).toBe(5);
  });
});

// ── auto-refresh ──────────────────────────────────────────────────────────────

describe('useDataRefresh – auto-refresh callback', () => {
  it('invokes onRefresh after the interval', async () => {
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    renderHook(() =>
      useDataRefresh({ refreshIntervalMs: 5_000, onRefresh }),
    );

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });

    expect(onRefresh).toHaveBeenCalledTimes(1);
  });

  it('updates lastUpdated after a successful auto-refresh', async () => {
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() =>
      useDataRefresh({ refreshIntervalMs: 5_000, onRefresh }),
    );

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });

    expect(result.current.lastUpdated).not.toBeNull();
  });

  it('does not call onRefresh before the interval elapses', async () => {
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    renderHook(() =>
      useDataRefresh({ refreshIntervalMs: 10_000, onRefresh }),
    );

    act(() => {
      vi.advanceTimersByTime(9_000); // 1 s short
    });

    expect(onRefresh).not.toHaveBeenCalled();
  });
});

// ── manual refresh ────────────────────────────────────────────────────────────

describe('useDataRefresh – triggerRefresh', () => {
  it('sets isRefreshing to true then false', async () => {
    let resolveRefresh!: () => void;
    const onRefresh = vi.fn().mockImplementation(
      () => new Promise<void>((res) => { resolveRefresh = res; }),
    );

    const { result } = renderHook(() =>
      useDataRefresh({ onRefresh }),
    );

    act(() => {
      result.current.triggerRefresh();
    });

    expect(result.current.isRefreshing).toBe(true);

    await act(async () => {
      resolveRefresh();
    });

    expect(result.current.isRefreshing).toBe(false);
  });

  it('calls onRefresh when triggerRefresh is invoked', async () => {
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() =>
      useDataRefresh({ onRefresh }),
    );

    await act(async () => {
      await result.current.triggerRefresh();
    });

    expect(onRefresh).toHaveBeenCalledTimes(1);
  });

  it('updates lastUpdated after manual refresh', async () => {
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() =>
      useDataRefresh({ onRefresh }),
    );

    expect(result.current.lastUpdated).toBeNull();

    await act(async () => {
      await result.current.triggerRefresh();
    });

    expect(result.current.lastUpdated).not.toBeNull();
    expect(result.current.lastUpdated).toBeInstanceOf(Date);
  });

  it('does not call onRefresh if already refreshing', async () => {
    let resolveRefresh!: () => void;
    const onRefresh = vi.fn().mockImplementation(
      () => new Promise<void>((res) => { resolveRefresh = res; }),
    );

    const { result } = renderHook(() =>
      useDataRefresh({ onRefresh }),
    );

    // Start first refresh (won't resolve yet)
    act(() => {
      result.current.triggerRefresh();
    });

    // Try a second trigger while the first is in-flight
    await act(async () => {
      await result.current.triggerRefresh();
    });

    // onRefresh should only have been called once
    expect(onRefresh).toHaveBeenCalledTimes(1);

    // Cleanup
    await act(async () => {
      resolveRefresh();
    });
  });

  it('resets the countdown after manual trigger', async () => {
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() =>
      useDataRefresh({ refreshIntervalMs: 30_000, onRefresh }),
    );

    // Advance 10 s so countdown is 20
    act(() => {
      vi.advanceTimersByTime(10_000);
    });

    await act(async () => {
      await result.current.triggerRefresh();
    });

    // Countdown should be reset back to 30
    expect(result.current.secondsUntilRefresh).toBe(30);
  });
});

// ── markUpdated ───────────────────────────────────────────────────────────────

describe('useDataRefresh – markUpdated', () => {
  it('updates lastUpdated without triggering onRefresh', () => {
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() =>
      useDataRefresh({ onRefresh }),
    );

    expect(result.current.lastUpdated).toBeNull();

    act(() => {
      result.current.markUpdated();
    });

    expect(result.current.lastUpdated).not.toBeNull();
    expect(onRefresh).not.toHaveBeenCalled();
  });
});

// ── refreshOnMount ────────────────────────────────────────────────────────────

describe('useDataRefresh – refreshOnMount', () => {
  it('calls onRefresh immediately when refreshOnMount is true', async () => {
    const onRefresh = vi.fn().mockResolvedValue(undefined);

    renderHook(() =>
      useDataRefresh({ onRefresh, refreshOnMount: true }),
    );

    // Flush the async IIFE inside the mount useEffect.
    await act(async () => {});

    expect(onRefresh).toHaveBeenCalledTimes(1);
  });

  it('does NOT call onRefresh on mount when refreshOnMount is false (default)', async () => {
    const onRefresh = vi.fn().mockResolvedValue(undefined);

    renderHook(() =>
      useDataRefresh({ onRefresh, refreshOnMount: false }),
    );

    // Advance slightly to let any spurious calls settle
    act(() => { vi.advanceTimersByTime(100); });

    expect(onRefresh).not.toHaveBeenCalled();
  });
});
