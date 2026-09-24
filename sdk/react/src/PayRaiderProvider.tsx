import { createContext, useContext, useMemo, type ReactNode } from "react";
import { PayRaider, createClient, type PayRaiderConfig } from "@payraider/sdk";

const PayRaiderContext = createContext<PayRaider | null>(null);

export interface PayRaiderProviderProps {
  children: ReactNode;
  /** Use an already-constructed client — takes precedence over `network`/`config`. */
  client?: PayRaider;
  /** Convenience path: build a client for "mainnet" or "testnet". */
  network?: "mainnet" | "testnet";
  config?: Omit<PayRaiderConfig, "baseUrl">;
}

/**
 * Wrap your app (or the part of it that renders PayRaider data) in
 * this provider so `use*` hooks and components from this package can find a
 * client. Pass either a pre-built `client`, or `network` (+ optional
 * `config`) and one will be created for you.
 */
export function PayRaiderProvider({
  children,
  client,
  network = "mainnet",
  config,
}: PayRaiderProviderProps) {
  const resolvedClient = useMemo(
    () => client ?? createClient(network, config),
    // eslint-disable-next-line react-hooks/exhaustive-deps -- config is expected to be stable/memoized by the caller
    [client, network],
  );

  return (
    <PayRaiderContext.Provider value={resolvedClient}>
      {children}
    </PayRaiderContext.Provider>
  );
}

export function usePayRaiderClient(): PayRaider {
  const client = useContext(PayRaiderContext);
  if (!client) {
    throw new Error(
      "usePayRaiderClient must be used within a <PayRaiderProvider>",
    );
  }
  return client;
}
