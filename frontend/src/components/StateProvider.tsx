"use client";

import React from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { ReactQueryLogger } from '@/lib/react-query/logger';
import { ReactQueryProvider as CustomReactQueryProvider } from '@/lib/react-query/provider';
import { useAppStore } from '@/lib/zustand/store';

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
      (window as any).__stateDevtools = {
        // Store debugging
        store,
        logState: () => console.log('Store State:', useAppStore.getState()),
        resetState: () => store.resetState(),

        // Query debugging
        logQueries: () => {
          const cache = queryClient.getQueryCache();
          console.log('Query Cache:', cache.getAll());
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
            activeQueries: queries.filter((q: any) => q.getObserversCount() > 0).length,
            staleQueries: queries.filter((q: any) => q.isStale()).length,
            averageStaleTime: queries.reduce((acc: number, q: any) => acc + q.state.staleTime, 0) / queries.length,
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
      const debugTools = (window as any).__stateDevtools;

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
