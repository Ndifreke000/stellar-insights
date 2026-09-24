"use client";

import React from 'react';
import {
  QueryCache,
  QueryClient,
  QueryClientProvider,
  isServer,
} from '@tanstack/react-query';
import { logger } from '@/lib/logger';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { ReactQueryLogger } from './logger';

/** Default freshness for server data. Override per-hook for volatile data. */
export const DEFAULT_STALE_TIME = 5 * 60 * 1000;

export function createQueryClient(): QueryClient {
  return new QueryClient({
    // Central error logging for every query, so components don't each need to.
    queryCache: new QueryCache({
      onError: (error, query) => {
        // status 0 = backend unreachable; pages fall back to demo data, so don't spam.
        if ((error as { status?: number }).status === 0) return;
        logger.error('Query failed', {
          queryKey: query.queryKey,
          error: error instanceof Error ? error.message : String(error),
          requestId: (error as { requestId?: string }).requestId,
        });
      },
    }),
    defaultOptions: {
      queries: {
        staleTime: DEFAULT_STALE_TIME,

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

let browserQueryClient: QueryClient | undefined;

/**
 * Returns the query client for the current environment.
 *
 * On the server a fresh client is created per request so data never leaks
 * between users. In the browser a single client is reused for the lifetime of
 * the tab so the cache (and request de-duplication) survives re-renders and
 * Suspense boundaries.
 */
export function getQueryClient(): QueryClient {
  if (isServer) return createQueryClient();
  browserQueryClient ??= createQueryClient();
  return browserQueryClient;
}

interface ReactQueryProviderProps {
  children: React.ReactNode;
  client?: QueryClient;
}

export function ReactQueryProvider({ children, client }: ReactQueryProviderProps) {
  // Lazily initialised once per mount — never recreate the client on render,
  // or every re-render would discard the cache and refetch everything.
  const [queryClient] = React.useState(() => client ?? getQueryClient());

  return (
    <QueryClientProvider client={queryClient}>
      {children}
      {process.env.NODE_ENV === 'development' && (
        <>
          <ReactQueryDevtools initialIsOpen={false} buttonPosition="bottom-left" />
          <ReactQueryLogger />
        </>
      )}
    </QueryClientProvider>
  );
}
