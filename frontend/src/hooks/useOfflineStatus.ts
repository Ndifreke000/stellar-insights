'use client';

import { useEffect, useState, useCallback } from 'react';

export interface OfflineStatus {
  isOnline: boolean;
  /** True if the user was previously offline and just came back */
  justReconnected: boolean;
  /** Timestamp when the connection was lost, null if currently online */
  offlineSince: Date | null;
}

/**
 * Lightweight hook that tracks online/offline status.
 * Separate from useProgressiveWebApp so any component can consume it
 * without pulling in the full PWA installation machinery.
 */
export function useOfflineStatus(): OfflineStatus {
  // Guard on `window`, not `navigator`: Node.js ships a built-in global
  // `navigator` (for fetch-spec compatibility) that has no `onLine`
  // property, so `typeof navigator !== 'undefined'` is true during SSR too.
  // That made `navigator.onLine` evaluate to `undefined` (falsy) on the
  // server, rendering the offline banner on every SSR pass and causing a
  // hydration mismatch as soon as the real browser value took over.
  const [isOnline, setIsOnline] = useState(() =>
    typeof window !== 'undefined' ? navigator.onLine : true,
  );
  const [justReconnected, setJustReconnected] = useState(false);
  const [offlineSince, setOfflineSince] = useState<Date | null>(null);

  const handleOnline = useCallback(() => {
    setIsOnline(true);
    setOfflineSince(null);
    setJustReconnected(true);
    // Clear the "just reconnected" flag after 4 s
    setTimeout(() => setJustReconnected(false), 4000);
  }, []);

  const handleOffline = useCallback(() => {
    setIsOnline(false);
    setOfflineSince(new Date());
    setJustReconnected(false);
  }, []);

  useEffect(() => {
    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);
    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, [handleOnline, handleOffline]);

  return { isOnline, justReconnected, offlineSince };
}
