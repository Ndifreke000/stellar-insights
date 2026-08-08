import { useEffect, useState, useCallback } from "react";
import type { CorridorDetail } from "@stellar-insights/sdk";
import { useStellarInsightsClient } from "../StellarInsightsProvider.js";

export interface UseCorridorResult {
  data: CorridorDetail | null;
  error: Error | null;
  isLoading: boolean;
  refetch: () => void;
}

/**
 * Fetches reliability/liquidity detail for a single payment corridor
 * (e.g. `useCorridor("USDC", "BRL")`).
 */
export function useCorridor(source: string, destination: string): UseCorridorResult {
  const client = useStellarInsightsClient();
  const [data, setData] = useState<CorridorDetail | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [refetchToken, setRefetchToken] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setError(null);

    client.corridors
      .get(source, destination)
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
  }, [client, source, destination, refetchToken]);

  const refetch = useCallback(() => setRefetchToken((t) => t + 1), []);

  return { data, error, isLoading, refetch };
}
