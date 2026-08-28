"use client";

import React from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { logger } from '@/lib/logger';

/**
 * React Query Logger Component
 * Logs query events for debugging and monitoring
 */
export function ReactQueryLogger() {
  const queryClient = useQueryClient();

  React.useEffect(() => {
    // Simple cache monitoring without event listeners
    const interval = setInterval(() => {
      const cache = queryClient.getQueryCache();
      const queries = cache.getAll();

      const errorQueries = queries.filter(q => q.state.status === 'error');
      if (errorQueries.length > 0) {
        logger.warn('Queries with errors', {
          count: errorQueries.length,
          queries: errorQueries.map(q => ({
            queryKey: q.queryKey,
            error: q.state.error,
          })),
        });
      }
    }, 30000); // Check every 30 seconds

    return () => clearInterval(interval);
  }, [queryClient]);

  // This component doesn't render anything
  return null;
}

/**
 * Custom query logger for debugging
 */
export function useQueryLogger() {
  const queryClient = useQueryClient();

  const logQueryState = React.useCallback(() => {
    const cache = queryClient.getQueryCache();
    const queries = cache.getAll();

    logger.info('Query Cache State', {
      totalQueries: queries.length,
      activeQueries: queries.filter(q => q.getObserversCount() > 0).length,
      staleQueries: queries.filter(q => q.isStale()).length,
      errorQueries: queries.filter(q => q.state.status === 'error').length,
    });
  }, [queryClient]);

  const logCacheStats = React.useCallback(() => {
    const cache = queryClient.getQueryCache();
    const queries = cache.getAll();

    const stats = {
      total: queries.length,
      active: queries.filter(q => q.getObserversCount() > 0).length,
      stale: queries.filter(q => q.isStale()).length,
      inactive: queries.filter(q => q.getObserversCount() === 0).length,
      errors: queries.filter(q => q.state.status === 'error').length,
      fetching: queries.filter(q => q.state.fetchStatus === 'fetching').length,
    };

    logger.info('Query Cache Statistics', stats);
    return stats;
  }, [queryClient]);

  const invalidateQueries = React.useCallback(() => {
    const invalidated = queryClient.invalidateQueries();

    logger.info('All queries invalidated');

    return invalidated;
  }, [queryClient]);

  return {
    logQueryState,
    logCacheStats,
    invalidateQueries,
  };
}

/**
 * Development-only query monitoring hook
 */
export function useQueryMonitor() {
  const { logCacheStats } = useQueryLogger();
  const queryClient = useQueryClient();

  React.useEffect(() => {
    // Only run in development
    if (process.env.NODE_ENV !== 'development') {
      return;
    }

    // Log cache stats every 30 seconds
    const interval = setInterval(() => {
      logCacheStats();
    }, 30000);

    return () => clearInterval(interval);
  }, [logCacheStats]);

  // expose debug functions to window for manual debugging
  React.useEffect(() => {
    if (typeof window !== 'undefined' && process.env.NODE_ENV === 'development') {
      (window as unknown as { __queryMonitor: unknown }).__queryMonitor = {
        logStats: logCacheStats,
        logState: () => {
          const cache = queryClient.getQueryCache();
          const queries = cache.getAll();
          logger.info('All Queries', {
            totalQueries: queries.length,
            queryKeys: queries.map((q) => q.queryKey)
          });
        },
      };
    }
  }, [logCacheStats, queryClient]);
}
