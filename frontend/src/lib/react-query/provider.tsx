"use client";

import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { ReactQueryLogger } from './logger';

// Create a client
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        // Keep cached data fresh for 5 minutes
        staleTime: 5 * 60 * 1000,

        // Offline mode (#1489): hold cached data for 24 h so pages remain
        // usable after extended offline periods.
        gcTime: 24 * 60 * 60 * 1000,

        // Use cached data when offline; only fetch when a network is available.
        // 'offlineFirst' returns cached data immediately and retries in the
        // background once connectivity is restored.
        networkMode: 'offlineFirst',

        retry: (failureCount, error) => {
          if (error && typeof error === 'object' && 'status' in error) {
            const status = (error as { status: number }).status;
            if (status >= 400 && status < 500) return false;
          }
          return failureCount < 3;
        },

        retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000),
        refetchOnWindowFocus: false,
        refetchOnReconnect: true, // auto-refresh stale queries when back online
        refetchInterval: false,
        throwOnError: false,
      },
      mutations: {
        // Also use offlineFirst so mutations queue rather than fail immediately
        networkMode: 'offlineFirst',
        retry: 1,
        retryDelay: 1000,
      },
    },
  });
}

interface ReactQueryProviderProps {
  children: React.ReactNode;
  client?: QueryClient;
}

export function ReactQueryProvider({ children, client }: ReactQueryProviderProps) {
  const queryClient = client || createQueryClient();

  return (
    <QueryClientProvider client={queryClient}>
      {children}
      <ReactQueryDevtools initialIsOpen={false} />
      <ReactQueryLogger />
    </QueryClientProvider>
  );
}

// Export a singleton instance for easy access
export const queryClient = createQueryClient();
