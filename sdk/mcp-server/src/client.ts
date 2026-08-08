import { PayRaider, NETWORKS } from "@payraider/sdk";
import type { PayRaiderClient } from "./sdk-types.js";

/**
 * One client per server process, matching how every other SDK consumer
 * authenticates. PAYRAIDER_BASE_URL overrides the network default,
 * for pointing at a local/dev backend.
 */
export function createClientFromEnv(): PayRaiderClient {
  const apiKey = process.env.PAYRAIDER_API_KEY;
  if (!apiKey) {
    throw new Error(
      "PAYRAIDER_API_KEY is required. Set it to a PayRaider API key " +
        "(see /api/api-keys) before starting the MCP server.",
    );
  }

  const network = process.env.PAYRAIDER_NETWORK === "mainnet" ? "mainnet" : "testnet";
  const baseUrl = process.env.PAYRAIDER_BASE_URL ?? NETWORKS[network].apiBaseUrl;

  return new PayRaider({ apiKey, baseUrl });
}

export const ALLOW_WRITES = process.env.PAYRAIDER_MCP_ALLOW_WRITES === "true";
