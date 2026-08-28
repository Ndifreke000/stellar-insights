import { StellarInsights, NETWORKS } from "@stellar-insights/sdk";
import type { StellarInsightsClient } from "./sdk-types.js";

/**
 * One client per server process, matching how every other SDK consumer
 * authenticates. STELLAR_INSIGHTS_BASE_URL overrides the network default,
 * for pointing at a local/dev backend.
 */
export function createClientFromEnv(): StellarInsightsClient {
  const apiKey = process.env.STELLAR_INSIGHTS_API_KEY;
  if (!apiKey) {
    throw new Error(
      "STELLAR_INSIGHTS_API_KEY is required. Set it to a Stellar Insights API key " +
        "(see /api/api-keys) before starting the MCP server.",
    );
  }

  const network = process.env.STELLAR_INSIGHTS_NETWORK === "mainnet" ? "mainnet" : "testnet";
  const baseUrl = process.env.STELLAR_INSIGHTS_BASE_URL ?? NETWORKS[network].apiBaseUrl;

  return new StellarInsights({ apiKey, baseUrl });
}

export const ALLOW_WRITES = process.env.STELLAR_INSIGHTS_MCP_ALLOW_WRITES === "true";
