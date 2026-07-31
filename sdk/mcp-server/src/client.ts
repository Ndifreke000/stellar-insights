import { StellarInsights, NETWORKS } from "@stellar-insights/sdk";
import type { StellarInsightsClient } from "./sdk-types.js";

/** Deployed per docs/testnet-quickstart.md and contracts/.env.testnet - no mainnet deployment exists yet. */
const TESTNET_SNAPSHOT_CONTRACT_ID = "CAPHQZ4BBT43HU5EUSJAOPKWB66HGLTN4AKJUALV3R2RXS4A6IOXWUTL";

export function getNetwork(): "mainnet" | "testnet" {
  return process.env.STELLAR_INSIGHTS_NETWORK === "mainnet" ? "mainnet" : "testnet";
}

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

  const network = getNetwork();
  const baseUrl = process.env.STELLAR_INSIGHTS_BASE_URL ?? NETWORKS[network].apiBaseUrl;

  return new StellarInsights({ apiKey, baseUrl });
}

export interface SorobanConfig {
  rpcUrl: string;
  networkPassphrase: string;
  contractId: string;
}

/** Throws only when actually called (not at server startup) so other tools keep working without Soroban configured. */
export function getSorobanConfigFromEnv(): SorobanConfig {
  const network = getNetwork();
  const rpcUrl = process.env.STELLAR_INSIGHTS_SOROBAN_RPC_URL ?? NETWORKS[network].rpcUrl;
  const networkPassphrase = NETWORKS[network].networkPassphrase;
  const contractId =
    process.env.STELLAR_INSIGHTS_SNAPSHOT_CONTRACT_ID ??
    (network === "testnet" ? TESTNET_SNAPSHOT_CONTRACT_ID : undefined);

  if (!contractId) {
    throw new Error(
      "STELLAR_INSIGHTS_SNAPSHOT_CONTRACT_ID is required on mainnet (no default deployment is known). " +
        "On testnet it defaults to the address in contracts/.env.testnet.",
    );
  }

  return { rpcUrl, networkPassphrase, contractId };
}

export const ALLOW_WRITES = process.env.STELLAR_INSIGHTS_MCP_ALLOW_WRITES === "true";
