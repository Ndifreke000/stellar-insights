import React from 'react';
import {
  useQuery,
  useMutation,
  useQueryClient,
  useInfiniteQuery,
  useIsFetching as useRQIsFetching,
  type QueryKey,
} from '@tanstack/react-query';
import { logger } from '@/lib/logger';

export { queryKeys } from './keys';

/**
 * Generic query hook. Errors are logged centrally by the QueryCache in
 * `./provider`.
 *
 * Prefer the resource-specific hooks in `./queries` in components; use this
 * only when adding a new resource hook.
 */
export function useApiQuery<T>(
  queryKey: readonly unknown[],
  queryFn: () => Promise<T>,
  options?: {
    staleTime?: number;
    refetchInterval?: number;
    enabled?: boolean;
    /** `false` disables retries; a number caps them; omit to use the client default. */
    retry?: boolean | number;
  }
) {
  return useQuery({
    queryKey,
    queryFn,
    staleTime: options?.staleTime,
    refetchInterval: options?.refetchInterval,
    enabled: options?.enabled,
    // `retry: true` would mean "retry forever" in React Query, so only forward
    // explicit values and otherwise defer to the client's status-aware policy.
    ...(options?.retry !== undefined && options.retry !== true
      ? { retry: options.retry }
      : {}),
  });
}

/**
 * Generic mutation hook with success/error handling and cache invalidation.
 */
export function useApiMutation<T, V>(
  mutationFn: (variables: V) => Promise<T>,
  options?: {
    onSuccess?: (data: T, variables: V) => void;
    onError?: (error: Error, variables: V) => void;
    invalidateQueries?: readonly (readonly unknown[])[];
  }
) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (variables: V) => {
      try {
        return await mutationFn(variables);
      } catch (error) {
        // Variables may contain credentials (JWTs, account keys) — don't log them.
        logger.error('Mutation failed', {
          error: error instanceof Error ? error.message : 'Unknown error',
        });
        throw error;
      }
    },
    onSuccess: async (data, variables) => {
      if (options?.invalidateQueries) {
        await Promise.all(
          options.invalidateQueries.map((queryKey) =>
            queryClient.invalidateQueries({ queryKey })
          )
        );
      }
      options?.onSuccess?.(data, variables);
    },
    onError: options?.onError,
  });
}

/**
 * Infinite query hook for cursor-paginated endpoints.
 * Pair with `getNextCursor` from `@/lib/api/pagination`.
 */
export function useApiInfiniteQuery<T, P = string | undefined>(
  queryKey: readonly unknown[],
  queryFn: ({ pageParam }: { pageParam: P }) => Promise<T>,
  options: {
    initialPageParam: P;
    getNextPageParam: (lastPage: T) => P | undefined | null;
    enabled?: boolean;
  }
) {
  return useInfiniteQuery({
    queryKey,
    queryFn: ({ pageParam }) => queryFn({ pageParam: pageParam as P }),
    initialPageParam: options.initialPageParam,
    getNextPageParam: options.getNextPageParam,
    enabled: options.enabled,
  });
}

/**
 * Returns a stable function that prefetches a query into the cache.
 */
export function usePrefetchQuery() {
  const queryClient = useQueryClient();

  return React.useCallback(
    <T,>(queryKey: readonly unknown[], queryFn: () => Promise<T>) =>
      queryClient.prefetchQuery({
        queryKey,
        queryFn,
        staleTime: 5 * 60 * 1000,
      }),
    [queryClient]
  );
}

/**
 * Returns a stable function that invalidates every query under `queryKey`.
 */
export function useInvalidateQueries() {
  const queryClient = useQueryClient();

  return React.useCallback(
    (queryKey: readonly unknown[]) => queryClient.invalidateQueries({ queryKey }),
    [queryClient]
  );
}

/**
 * Returns a stable function that resets the entire query cache.
 */
export function useResetQueries() {
  const queryClient = useQueryClient();

  return React.useCallback(() => queryClient.resetQueries(), [queryClient]);
}

/**
 * Number of in-flight fetches, optionally scoped to `queryKey`.
 */
export function useIsFetching(queryKey?: readonly unknown[]) {
  return useRQIsFetching(queryKey ? { queryKey: queryKey as QueryKey } : undefined);
}

/**
 * Whether the cached query under `queryKey` is stale. Re-evaluates whenever the
 * query cache changes; never fetches or writes to the cache itself.
 */
export function useIsStale(queryKey: readonly unknown[]) {
  const queryClient = useQueryClient();
  const hash = JSON.stringify(queryKey);

  const getSnapshot = React.useCallback(
    () =>
      queryClient.getQueryCache().find({ queryKey: queryKey as QueryKey, exact: true })?.isStale() ??
      false,
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [queryClient, hash]
  );

  const subscribe = React.useCallback(
    (onChange: () => void) => queryClient.getQueryCache().subscribe(onChange),
    [queryClient]
  );

  return React.useSyncExternalStore(subscribe, getSnapshot, () => false);
}
