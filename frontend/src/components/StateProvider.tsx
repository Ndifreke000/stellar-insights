"use client";

import React from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { ReactQueryProvider } from '@/lib/react-query/provider';
import { useAppStore } from '@/lib/zustand/store';
import { logger } from '@/lib/logger';

interface StateDevtoolsObject {
  getState: () => ReturnType<typeof useAppStore.getState>;
  logState: () => void;
  resetState: () => void;
  logQueries: () => void;
  invalidateAll: () => Promise<void>;
  getQueryData: (queryKey: readonly unknown[]) => unknown;
  getPerformance: () => {
    totalQueries: number;
    activeQueries: number;
    staleQueries: number;
    fetchingQueries: number;
  };
}

declare global {
  interface Window {
    __stateDevtools?: StateDevtoolsObject;
  }
}

interface StateProviderProps {
  children: React.ReactNode;
}

/**
 * Root state provider: React Query for server state, Zustand for client state.
 * React Query devtools and the query logger are mounted by `ReactQueryProvider`
 * in development; this adds `window.__stateDevtools` for console debugging.
 *
 * See docs/FRONTEND_STATE_MANAGEMENT.md.
 */
export function StateProvider({ children }: StateProviderProps) {
  return (
    <ReactQueryProvider>
      {children}
      {process.env.NODE_ENV === 'development' && <StateDevTools />}
    </ReactQueryProvider>
  );
}

/**
 * Development-only console helpers. Reads the store imperatively so it never
 * subscribes to (and re-renders on) state changes.
 */
function StateDevTools() {
  const queryClient = useQueryClient();

  React.useEffect(() => {
    window.__stateDevtools = {
      // Client state (Zustand)
      getState: () => useAppStore.getState(),
      logState: () => logger.debug('Store State:', { state: useAppStore.getState() }),
      resetState: () => useAppStore.getState().resetState(),

      // Server state (React Query)
      logQueries: () =>
        logger.debug('Query Cache:', { queries: queryClient.getQueryCache().getAll() }),
      invalidateAll: () => queryClient.invalidateQueries(),
      getQueryData: (queryKey: readonly unknown[]) => queryClient.getQueryData(queryKey),
      getPerformance: () => {
        const queries = queryClient.getQueryCache().getAll();
        return {
          totalQueries: queries.length,
          activeQueries: queries.filter((q) => q.getObserversCount() > 0).length,
          staleQueries: queries.filter((q) => q.isStale()).length,
          fetchingQueries: queries.filter((q) => q.state.fetchStatus === 'fetching').length,
        };
      },
    };

    return () => {
      delete window.__stateDevtools;
    };
  }, [queryClient]);

  return null;
}

export { StateDevTools };
