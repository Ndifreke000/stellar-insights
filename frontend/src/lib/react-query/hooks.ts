import { useCallback } from 'react';
import { useQuery, useMutation, useQueryClient, useInfiniteQuery } from '@tanstack/react-query';
import { logger } from '@/lib/logger';

// Define query key factory for consistent keys
export const queryKeys = {
  // Anchors
  anchors: ['anchors'] as const,
  anchor: (id: string) => ['anchors', id] as const,
  anchorAssets: (id: string) => ['anchors', id, 'assets'] as const,
  
  // Corridors
  corridors: ['corridors'] as const,
  corridor: (id: string) => ['corridors', id] as const,
  corridorMetrics: (id: string) => ['corridors', id, 'metrics'] as const,
  
  // SEP-24
  sep24Anchors: ['sep24', 'anchors'] as const,
  sep24Info: (server: string) => ['sep24', 'info', server] as const,
  sep24Transactions: (server: string) => ['sep24', 'transactions', server] as const,
  
  // SEP-31
  sep31Anchors: ['sep31', 'anchors'] as const,
  sep31Info: (server: string) => ['sep31', 'info', server] as const,
  sep31Transactions: (server: string) => ['sep31', 'transactions', server] as const,
  
  // RPC
  rpcHealth: ['rpc', 'health'] as const,
  rpcLedger: ['rpc', 'ledger'] as const,
  rpcPayments: (account?: string) => ['rpc', 'payments', account] as const,
  rpcTrades: ['rpc', 'trades'] as const,
  rpcOrderbook: ['rpc', 'orderbook'] as const,
  
  // Jobs
  jobStatus: ['jobs', 'status'] as const,
  jobHealth: ['jobs', 'health'] as const,
  jobMetrics: ['jobs', 'metrics'] as const,
  
  // Cache stats
  cacheStats: ['cache', 'stats'] as const,
  
  // Price feeds
  priceFeeds: ['price-feeds'] as const,
  priceFeed: (id: string) => ['price-feeds', id] as const,
} as const;

/**
 * Generic query hook with error handling
 */
export function useApiQuery<T>(
  queryKey: readonly unknown[],
  queryFn: () => Promise<T>,
  options?: {
    staleTime?: number;
    refetchInterval?: number;
    enabled?: boolean;
    retry?: boolean | number;
  }
) {
  return useQuery({
    queryKey,
    queryFn: async () => {
      try {
        const result = await queryFn();
        return result;
      } catch (error) {
        logger.error('Query failed', {
          queryKey,
          error: error instanceof Error ? error.message : 'Unknown error',
        });
        throw error;
      }
    },
    staleTime: options?.staleTime,
    refetchInterval: options?.refetchInterval,
    enabled: options?.enabled,
    retry: options?.retry !== false,
  });
}

/**
 * Generic mutation hook with success/error handling
 */
export function useApiMutation<T, V>(
  mutationFn: (variables: V) => Promise<T>,
  options?: {
    onSuccess?: (data: T, variables: V) => void;
    onError?: (error: Error, variables: V) => void;
    invalidateQueries?: readonly unknown[][];
  }
) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (variables: V) => {
      try {
        const result = await mutationFn(variables);
        logger.info('Mutation successful', {
          variables,
        });
        return result;
      } catch (error) {
        logger.error('Mutation failed', {
          variables,
          error: error instanceof Error ? error.message : 'Unknown error',
        });
        throw error;
      }
    },
    onSuccess: (data, variables) => {
      // Invalidate related queries
      if (options?.invalidateQueries) {
        options.invalidateQueries.forEach(queryKey => {
          queryClient.invalidateQueries({ queryKey });
        });
      }
      
      // Call custom success handler
      options?.onSuccess?.(data, variables);
    },
    onError: options?.onError,
  });
}

/**
 * Infinite query hook for paginated data
 */
export function useApiInfiniteQuery<T>(
  queryKey: readonly unknown[],
  queryFn: ({ pageParam }: { pageParam?: unknown }) => Promise<T>,
  options: {
    initialPageParam?: unknown;
    getNextPageParam: (lastPage: T) => unknown;
    enabled?: boolean;
  }
) {
  return useInfiniteQuery({
    queryKey,
    queryFn,
    initialPageParam: options?.initialPageParam,
    getNextPageParam: options?.getNextPageParam,
    enabled: options?.enabled,
  });
}

/**
 * Hook for prefetching data
 */
export function usePrefetchQuery() {
  const queryClient = useQueryClient();

  return useCallback(
    <T>(queryKey: readonly unknown[], queryFn: () => Promise<T>) => {
      queryClient.prefetchQuery({
        queryKey,
        queryFn,
        staleTime: 5 * 60 * 1000, // 5 minutes
      });
    },
    [queryClient]
  );
}

/**
 * Hook for invalidating queries
 */
export function useInvalidateQueries() {
  const queryClient = useQueryClient();

  return useCallback(
    (queryKey: readonly unknown[]) => {
      queryClient.invalidateQueries({ queryKey });
    },
    [queryClient]
  );
}

/**
 * Hook for resetting queries
 */
export function useResetQueries() {
  const queryClient = useQueryClient();

  return useCallback(() => {
    queryClient.resetQueries();
  }, [queryClient]);
}

/**
 * Hook for checking if query is fetching
 */
export function useIsFetching(queryKey?: readonly unknown[]) {
  const queryClient = useQueryClient();
  return useQuery({
    queryKey: queryKey || ['fetching'],
    queryFn: () => false,
    select: () => {
      return queryClient.isFetching(queryKey ? { queryKey } : undefined);
    },
    refetchInterval: 1000, // Check every second
  });
}

/**
 * Hook for checking if query is stale
 */
export function useIsStale(queryKey: readonly unknown[]) {
  const queryClient = useQueryClient();
  return useQuery({
    queryKey,
    queryFn: () => false,
    select: () => {
      const query = queryClient.getQueryCache().find({ queryKey });
      return query?.isStale() ?? false;
    },
    refetchInterval: 5000, // Check every 5 seconds
  });
}
