import { z } from "zod";
import type { StellarInsightsClient } from "./sdk-types.js";

const pagination = {
  page: z.number().int().min(1).optional().describe("Page number, 1-indexed"),
  limit: z.number().int().min(1).max(200).optional().describe("Results per page"),
  sort: z.string().optional().describe("Field to sort by"),
  order: z.enum(["asc", "desc"]).optional(),
};

export interface ToolDef {
  name: string;
  description: string;
  schema: z.ZodRawShape;
  call: (client: StellarInsightsClient, args: Record<string, unknown>) => Promise<unknown>;
}

/**
 * Read-only tool set (v1). Every tool maps 1:1 onto an existing
 * @stellar-insights/sdk resource method — no new backend behaviour.
 * Mutating operations (transactions, governance votes, webhooks, api keys,
 * alert rule writes, auth) are intentionally excluded; see README for the
 * write-tool opt-in.
 */
export const READ_ONLY_TOOLS: ToolDef[] = [
  {
    name: "list_corridors",
    description: "List Stellar payment corridors (directional asset pairs) with health, success rate and volume.",
    schema: pagination,
    call: (c, a) => c.corridors.list(a),
  },
  {
    name: "get_corridor",
    description: "Get detail for one corridor, including historical success-rate and latency data.",
    schema: {
      source: z.string().describe("Source asset, e.g. 'USDC:issuer' or 'native'"),
      destination: z.string().describe("Destination asset"),
    },
    call: (c, a) => c.corridors.get(a.source as string, a.destination as string),
  },
  {
    name: "list_anchors",
    description: "List Stellar anchor operators with health scores and supported assets.",
    schema: pagination,
    call: (c, a) => c.anchors.list(a),
  },
  {
    name: "get_anchor",
    description: "Get detail for one anchor by ID.",
    schema: { id: z.string() },
    call: (c, a) => c.anchors.get(a.id as string),
  },
  {
    name: "get_anchor_by_account",
    description: "Look up an anchor by its Stellar account address.",
    schema: { account: z.string().describe("Stellar account (G...) address") },
    call: (c, a) => c.anchors.getByAccount(a.account as string),
  },
  {
    name: "list_prices",
    description: "List current prices for all tracked assets.",
    schema: {},
    call: (c) => c.prices.list(),
  },
  {
    name: "get_price",
    description: "Get the current price for one asset.",
    schema: { asset: z.string() },
    call: (c, a) => c.prices.get(a.asset as string),
  },
  {
    name: "convert_price",
    description: "Convert an amount between two assets using current price data.",
    schema: {
      from: z.string(),
      to: z.string(),
      amount: z.number().positive(),
    },
    call: (c, a) => c.prices.convert(a.from as string, a.to as string, a.amount as number),
  },
  {
    name: "estimate_transfer_cost",
    description: "Estimate the total cost (fees) of transferring an amount between two assets.",
    schema: {
      source_asset: z.string(),
      destination_asset: z.string(),
      amount: z.number().positive(),
    },
    call: (c, a) =>
      c.costCalculator.estimate({
        source_asset: a.source_asset as string,
        destination_asset: a.destination_asset as string,
        amount: a.amount as number,
      }),
  },
  {
    name: "list_liquidity_pools",
    description: "List Stellar AMM liquidity pools with composition and volume.",
    schema: pagination,
    call: (c, a) => c.liquidityPools.list(a),
  },
  {
    name: "get_liquidity_pool",
    description: "Get detail for one liquidity pool by ID.",
    schema: { id: z.string() },
    call: (c, a) => c.liquidityPools.get(a.id as string),
  },
  {
    name: "get_network_info",
    description: "Get network-wide statistics for the currently configured Stellar network.",
    schema: {},
    call: (c) => c.network.info(),
  },
  {
    name: "list_available_networks",
    description: "List the Stellar networks (mainnet/testnet) this instance can report on.",
    schema: {},
    call: (c) => c.network.available(),
  },
  {
    name: "predict_payment_outcome",
    description: "Run the ML anomaly/outcome predictor for a payment or corridor scenario.",
    schema: { params: z.record(z.string(), z.unknown()).describe("Prediction input parameters") },
    call: (c, a) => c.ml.predict((a.params as Record<string, unknown>) ?? {}),
  },
  {
    name: "get_ml_status",
    description: "Get the status/health of the ML anomaly-detection model.",
    schema: {},
    call: (c) => c.ml.modelStatus(),
  },
  {
    name: "list_governance_proposals",
    description: "List on-chain governance proposals.",
    schema: pagination,
    call: (c, a) => c.governance.listProposals(a),
  },
  {
    name: "get_governance_proposal",
    description: "Get detail for one governance proposal by ID.",
    schema: { id: z.string() },
    call: (c, a) => c.governance.getProposal(a.id as string),
  },
  {
    name: "list_alert_history",
    description: "List past triggered alerts (read-only; does not create or modify alert rules).",
    schema: pagination,
    call: (c, a) => c.alerts.listHistory(a),
  },
  {
    name: "verify_asset",
    description: "Verify a Stellar asset (code + issuer) against the anchor registry and stellar.toml.",
    schema: { asset_code: z.string(), asset_issuer: z.string() },
    call: (c, a) => c.assetVerification.verify(a.asset_code as string, a.asset_issuer as string),
  },
  {
    name: "get_verified_asset",
    description: "Get a previously computed asset verification result.",
    schema: { asset_code: z.string(), asset_issuer: z.string() },
    call: (c, a) => c.assetVerification.get(a.asset_code as string, a.asset_issuer as string),
  },
  {
    name: "list_verified_assets",
    description: "List assets that have been verified against the anchor registry.",
    schema: pagination,
    call: (c, a) => c.assetVerification.list(a),
  },
];
