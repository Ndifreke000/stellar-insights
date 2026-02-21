import { useState, useEffect, useCallback, useRef } from "react";

export interface UseDataRefreshOptions {
  /** Auto-refresh interval in milliseconds. Defaults to 30 000 ms. */
  refreshIntervalMs?: number;
  /** Async callback invoked on each auto or manual refresh. */
  onRefresh?: () => Promise<void>;
  /** Whether to trigger an immediate refresh on mount. Defaults to false. */
  refreshOnMount?: boolean;
}

export interface UseDataRefreshReturn {
  /** Timestamp of the last successful data refresh. */
  lastUpdated: Date | null;
  /** Seconds remaining until the next auto-refresh. */
  secondsUntilRefresh: number;
  /** True while an in-flight refresh is pending. */
  isRefreshing: boolean;
  /** Call to immediately trigger a manual refresh and reset the countdown. */
  triggerRefresh: () => Promise<void>;
  /** Call to manually push the lastUpdated time (e.g., on a WS message). */
  markUpdated: () => void;
}

export function useDataRefresh({
  refreshIntervalMs = 30_000,
  onRefresh,
  refreshOnMount = false,
}: UseDataRefreshOptions = {}): UseDataRefreshReturn {
  const refreshIntervalSec = Math.max(1, Math.round(refreshIntervalMs / 1000));

  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [secondsUntilRefresh, setSecondsUntilRefresh] =
    useState(refreshIntervalSec);

  // Tracks elapsed seconds since the last countdown reset so we can rebuild
  // the countdown even when the component-level interval is re-created.
  const elapsedRef = useRef(0);
  const countdownIntervalRef = useRef<ReturnType<typeof setInterval> | null>(
    null,
  );
  const autoRefreshTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(
    null,
  );

  const clearTimers = useCallback(() => {
    if (countdownIntervalRef.current !== null) {
      clearInterval(countdownIntervalRef.current);
      countdownIntervalRef.current = null;
    }
    if (autoRefreshTimeoutRef.current !== null) {
      clearTimeout(autoRefreshTimeoutRef.current);
      autoRefreshTimeoutRef.current = null;
    }
  }, []);

  const startCountdown = useCallback(() => {
    clearTimers();
    elapsedRef.current = 0;
    setSecondsUntilRefresh(refreshIntervalSec);

    // Tick every second to update the visible countdown
    countdownIntervalRef.current = setInterval(() => {
      elapsedRef.current += 1;
      const remaining = Math.max(0, refreshIntervalSec - elapsedRef.current);
      setSecondsUntilRefresh(remaining);
    }, 1_000);

    // Schedule the auto-refresh at the full interval
    autoRefreshTimeoutRef.current = setTimeout(async () => {
      if (onRefresh) {
        setIsRefreshing(true);
        try {
          await onRefresh();
          setLastUpdated(new Date());
        } catch (err) {
          console.error("[useDataRefresh] Auto-refresh failed:", err);
        } finally {
          setIsRefreshing(false);
        }
      }
      // Restart the countdown after the refresh completes
      startCountdown();
    }, refreshIntervalMs);
  }, [clearTimers, onRefresh, refreshIntervalMs, refreshIntervalSec]);

  // Bootstrap: kick off the countdown on mount (and whenever the interval changes)
  useEffect(() => {
    startCountdown();
    return clearTimers;
  }, [startCountdown, clearTimers]);

  // Optional: fire an immediate refresh on mount
  useEffect(() => {
    if (!refreshOnMount || !onRefresh) return;
    (async () => {
      setIsRefreshing(true);
      try {
        await onRefresh();
        setLastUpdated(new Date());
      } catch (err) {
        console.error("[useDataRefresh] Mount refresh failed:", err);
      } finally {
        setIsRefreshing(false);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /** Manually trigger a refresh, reset the countdown. */
  const triggerRefresh = useCallback(async () => {
    if (isRefreshing) return;
    setIsRefreshing(true);
    try {
      if (onRefresh) await onRefresh();
      setLastUpdated(new Date());
    } catch (err) {
      console.error("[useDataRefresh] Manual refresh failed:", err);
    } finally {
      setIsRefreshing(false);
    }
    startCountdown();
  }, [isRefreshing, onRefresh, startCountdown]);

  /** Push a lastUpdated timestamp without triggering a full refresh cycle. */
  const markUpdated = useCallback(() => {
    setLastUpdated(new Date());
  }, []);

  return {
    lastUpdated,
    secondsUntilRefresh,
    isRefreshing,
    triggerRefresh,
    markUpdated,
  };
}
