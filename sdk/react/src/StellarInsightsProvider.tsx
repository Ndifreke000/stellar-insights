import { createContext, useContext, useMemo, type ReactNode } from "react";
import { StellarInsights, createClient, type StellarInsightsConfig } from "@stellar-insights/sdk";

const StellarInsightsContext = createContext<StellarInsights | null>(null);

export interface StellarInsightsProviderProps {
  children: ReactNode;
  /** Use an already-constructed client — takes precedence over `network`/`config`. */
  client?: StellarInsights;
  /** Convenience path: build a client for "mainnet" or "testnet". */
  network?: "mainnet" | "testnet";
  config?: Omit<StellarInsightsConfig, "baseUrl">;
}

/**
 * Wrap your app (or the part of it that renders Stellar Insights data) in
 * this provider so `use*` hooks and components from this package can find a
 * client. Pass either a pre-built `client`, or `network` (+ optional
 * `config`) and one will be created for you.
 */
export function StellarInsightsProvider({
  children,
  client,
  network = "mainnet",
  config,
}: StellarInsightsProviderProps) {
  const resolvedClient = useMemo(
    () => client ?? createClient(network, config),
    // eslint-disable-next-line react-hooks/exhaustive-deps -- config is expected to be stable/memoized by the caller
    [client, network],
  );

  return (
    <StellarInsightsContext.Provider value={resolvedClient}>
      {children}
    </StellarInsightsContext.Provider>
  );
}

export function useStellarInsightsClient(): StellarInsights {
  const client = useContext(StellarInsightsContext);
  if (!client) {
    throw new Error(
      "useStellarInsightsClient must be used within a <StellarInsightsProvider>",
    );
  }
  return client;
}
