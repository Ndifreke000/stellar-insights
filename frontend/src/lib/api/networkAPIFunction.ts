/**
 * Network API Functions
 */

import { api } from "./api";
import { NetworkInfo, SwitchNetworkResponse } from "./types";

/**
 * Get current network information
 */
export async function getCurrentNetwork(): Promise<NetworkInfo> {
  return api.get<NetworkInfo>("/network/info");
}

/**
 * Get all available networks
 */
export async function getAvailableNetworks(): Promise<NetworkInfo[]> {
  return api.get<NetworkInfo[]>("/network/available");
}

/**
 * Switch to a different network
 */
export async function switchNetwork(
  network: "mainnet" | "testnet",
): Promise<SwitchNetworkResponse> {
  return api.post<SwitchNetworkResponse>("/network/switch", { network });
}
