import { useEffect, useState, useCallback } from "react";
import type { Corridor, PaginatedResponse, PaginationParams } from "@stellar-insights/sdk";
import { useStellarInsightsClient } from "../StellarInsightsProvider.js";

export interface UseCorridorsResult {
  data: PaginatedResponse<Corridor> | null;
  error: Error | null;
  isLoading: boolean;
  refetch: () => void;
}

/** Fetches the list of payment corridors, optionally paginated. */
export function useCorridors(params?: PaginationParams): UseCorridorsResult {
  const client = useStellarInsightsClient();
  const [data, setData] = useState<PaginatedResponse<Corridor> | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [refetchToken, setRefetchToken] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setError(null);

    client.corridors
      .list(params)
      .then((result) => {
        if (!cancelled) setData(result);
      })
      .catch((err) => {
        if (!cancelled) setError(err instanceof Error ? err : new Error(String(err)));
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- params is expected to be stable/memoized by the caller
  }, [client, params, refetchToken]);

  const refetch = useCallback(() => setRefetchToken((t) => t + 1), []);

  return { data, error, isLoading, refetch };
}
