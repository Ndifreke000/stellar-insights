export type StellarNetwork = "mainnet" | "testnet";

export interface NetworkContextManagementParams {
  /** Override the active network for this context resolution */
  network?: StellarNetwork;
  /** Optional client name for observability */
  clientName?: string;
}

export interface NetworkContextManagementResult {
  success: true;
  network: StellarNetwork;
  data: {
    baseUrl: string;
    headers: Record<string, string>;
    environment: "browser" | "node" | "react-native";
    clientName?: string;
    configuredAt: string;
  };
}

export type NetworkChangeListener = (
  network: StellarNetwork,
  previousNetwork: StellarNetwork,
) => void;
