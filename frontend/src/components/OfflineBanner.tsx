'use client';

import React, { useEffect, useState } from 'react';
import { WifiOff, Wifi, X } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { useOfflineStatus } from '@/hooks/useOfflineStatus';

/**
 * Persistent banner shown at the top of the viewport when the user loses
 * internet connectivity. Automatically transitions to a "back online" state
 * and then dismisses itself.
 *
 * Wired into the locale layout so it appears on every page (#1489).
 */
export function OfflineBanner() {
  const { isOnline, justReconnected, offlineSince } = useOfflineStatus();
  const [dismissed, setDismissed] = useState(false);
  const [elapsed, setElapsed] = useState('');

  // Auto-dismiss the "back online" toast after 4 s
  useEffect(() => {
    if (justReconnected) {
      const t = setTimeout(() => setDismissed(true), 4000);
      return () => clearTimeout(t);
    }
  }, [justReconnected]);

  // Live "offline for X" counter
  useEffect(() => {
    if (isOnline || !offlineSince) return;
    const tick = () => {
      const secs = Math.floor((Date.now() - offlineSince.getTime()) / 1000);
      if (secs < 60) setElapsed(`${secs}s`);
      else setElapsed(`${Math.floor(secs / 60)}m ${secs % 60}s`);
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [isOnline, offlineSince]);

  // Reset dismissed when going offline, or dismiss when back online
  const visible = (!isOnline || justReconnected) && !(dismissed && isOnline && !justReconnected);

  return (
    <AnimatePresence>
      {visible && (
        <motion.div
          key={isOnline ? 'online' : 'offline'}
          initial={{ y: -56, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          exit={{ y: -56, opacity: 0 }}
          transition={{ type: 'spring', stiffness: 400, damping: 30 }}
          role="status"
          aria-live="polite"
          aria-atomic="true"
          className={`fixed top-0 left-0 right-0 z-[9999] flex items-center justify-between gap-3 px-4 py-2.5 text-sm font-medium ${
            isOnline
              ? 'bg-success/90 text-white'
              : 'bg-slate-900/95 border-b border-error/30 text-foreground'
          } backdrop-blur-md`}
        >
          <div className="flex items-center gap-2.5">
            {isOnline ? (
              <Wifi className="w-4 h-4 shrink-0 text-white" aria-hidden="true" />
            ) : (
              <WifiOff className="w-4 h-4 shrink-0 text-error" aria-hidden="true" />
            )}
            <span>
              {isOnline ? (
                'Back online — data is syncing'
              ) : (
                <>
                  <span className="text-error font-semibold">Offline</span>
                  {elapsed && (
                    <span className="text-muted-foreground ml-1.5 font-mono text-xs">
                      {elapsed}
                    </span>
                  )}
                  <span className="text-muted-foreground ml-2">
                    · Showing cached data
                  </span>
                </>
              )}
            </span>
          </div>

          <button
            onClick={() => setDismissed(true)}
            aria-label="Dismiss notification"
            className="p-1 rounded-md hover:bg-white/10 transition-colors shrink-0"
          >
            <X className="w-3.5 h-3.5" aria-hidden="true" />
          </button>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
