"use client";

import React from 'react';
import { useQueryClient, type Query } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { ReactQueryLogger } from '@/lib/react-query/logger';
import { ReactQueryProvider as CustomReactQueryProvider } from '@/lib/react-query/provider';
import { useAppStore } from '@/lib/zustand/store';
import { logger } from '@/lib/logger';

interface StateDevtoolsObject {
  store: ReturnType<typeof useAppStore>;
  logState: () => void;
  resetState: () => void;
  logQueries: () => void;
  invalidateAll: () => Promise<void>;
  getQueryData: (queryKey: string[]) => unknown;
  getPerformance: () => {
    totalQueries: number;
    activeQueries: number;
    staleQueries: number;
    averageStaleTime: number;
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

export function StateProvider({ children }: StateProviderProps) {
  return (
    <CustomReactQueryProvider>
      {children}
      <ReactQueryLogger />
      <ReactQueryDevtools initialIsOpen={false} />
      <StateDevTools />
    </CustomReactQueryProvider>
  );
}

/**
 * Development-only state debugging tools
 */
function StateDevTools() {
  const store = useAppStore();
  const queryClient = useQueryClient();

  React.useEffect(() => {
    if (process.env.NODE_ENV === 'development') {
      // Expose debugging functions to window
      window.__stateDevtools = {
        // Store debugging
        store,
        logState: () => logger.debug('Store State:', { state: useAppStore.getState() }),
        resetState: () => store.resetState(),

        // Query debugging
        logQueries: () => {
          const cache = queryClient.getQueryCache();
          logger.debug('Query Cache:', { queries: cache.getAll() });
        },
        invalidateAll: () => {
          queryClient.invalidateQueries();
        },
        getQueryData: (queryKey: string[]) => {
          const query = queryClient.getQueryCache().find({ queryKey });
          return query?.state.data;
        },

        // Performance debugging
        getPerformance: () => {
          const cache = queryClient.getQueryCache();
          const queries = cache.getAll();
          return {
            totalQueries: queries.length,
            activeQueries: queries.filter((q: Query) => q.getObserversCount() > 0).length,
            staleQueries: queries.filter((q: Query) => q.isStale()).length,
            averageStaleTime: queries.reduce((acc: number, q: Query) => acc + q.state.staleTime, 0) / queries.length,
          };
        },
      };
    }
  }, [store, queryClient]);

  return null; // This component doesn't render anything
}

/**
 * Hook for accessing debugging tools
 */
export function useDebugTools() {
  React.useEffect(() => {
    if (process.env.NODE_ENV === 'development') {
      const debugTools = window.__stateDevtools;

      // Log performance metrics every 30 seconds
      const interval = setInterval(() => {
        if (debugTools?.getPerformance) {
          const perf = debugTools.getPerformance();
          if (perf.staleQueries > 0) {
            console.warn(`Performance: ${perf.staleQueries} stale queries, average stale time: ${perf.averageStaleTime}ms`);
          }
        }
      }, 30000);

      return () => clearInterval(interval);
    }
  }, []);
}
