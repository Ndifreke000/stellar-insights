import { z } from "zod";
import type { PayRaiderClient } from "./sdk-types.js";

const pagination = {
  limit: z.number().int().min(1).max(200).optional().describe("Results per page (default 50)"),
  cursor: z
    .string()
    .optional()
    .describe("Opaque cursor from the previous page's pagination.next_cursor"),
};

const asset = (role: string) =>
  z.string().min(1).describe(`${role}: an asset code ("USDC") or "CODE:ISSUER"`);

export interface ToolDef {
  name: string;
  description: string;
  schema: z.ZodRawShape;
  /** True when the backend needs a signed-in user, not just an API key. */
  requiresAuth?: boolean;
  call: (client: PayRaiderClient, args: Record<string, unknown>) => Promise<unknown>;
}

/**
 * Read-only tool set. Every tool maps onto a @payraider/sdk resource method
 * that the backend actually serves. Mutating operations (transactions,
 * governance votes, webhooks, API keys, alert-rule writes, auth) are
 * intentionally excluded.
 */
export const READ_ONLY_TOOLS: ToolDef[] = [
  {
    name: "preflight_payment",
    description:
      "Check a payment corridor BEFORE paying out. Returns one decision " +
      "(proceed, caution, hold, or unknown when there is no recent data), the " +
      "checks behind it (success rate, liquidity headroom for the amount, p95 " +
      "latency, sample size, health score) and healthier alternative corridors " +
      "to the same destination asset. Works without an API key.",
    schema: {
      source_asset: asset("Asset the payment is sent in"),
      destination_asset: asset("Asset the recipient receives"),
      amount_usd: z
        .number()
        .positive()
        .optional()
        .describe("Payment size in USD; omit to skip the liquidity check"),
      min_success_rate: z
        .number()
        .min(0)
        .max(100)
        .optional()
        .describe("Minimum acceptable success rate in percent (default 95)"),
      max_p95_latency_ms: z
        .number()
        .positive()
        .optional()
        .describe("Maximum acceptable p95 latency in milliseconds (default 5000)"),
    },
    call: (c, a) =>
      c.preflight.check({
        source_asset: a.source_asset as string,
        destination_asset: a.destination_asset as string,
        amount_usd: a.amount_usd as number | undefined,
        min_success_rate: a.min_success_rate as number | undefined,
        max_p95_latency_ms: a.max_p95_latency_ms as number | undefined,
      }),
  },
  {
    name: "list_corridors",
    description:
      "List Stellar payment corridors (directional asset pairs) with success rate, latency, liquidity and health score. Each corridor's `id` is its corridor key.",
    schema: pagination,
    call: (c, a) => c.corridors.list(a),
  },
  {
    name: "get_corridor",
    description:
      "Get detail for one corridor: historical success rate, latency distribution, liquidity trend and related corridors.",
    schema: {
      corridor_key: z
        .string()
        .min(1)
        .describe('Corridor key from list_corridors, e.g. "USDC:G...->NGN:G..."'),
    },
    call: (c, a) => c.corridors.get(a.corridor_key as string),
  },
  {
    name: "list_anchors",
    description: "List Stellar anchor operators with reliability scores and supported assets.",
    schema: pagination,
    call: (c, a) => c.anchors.list(a),
  },
  {
    name: "get_anchor",
    description: "Get detail for one anchor by its ID (a UUID from list_anchors).",
    schema: { id: z.string().min(1) },
    call: (c, a) => c.anchors.get(a.id as string),
  },
  {
    name: "get_anchor_by_account",
    description: "Look up an anchor by its Stellar account address.",
    schema: { account: z.string().min(1).describe("Stellar account (G...) address") },
    call: (c, a) => c.anchors.getByAccount(a.account as string),
  },
  {
    name: "get_price",
    description: 'Get the current USD price of one asset, e.g. "XLM:native".',
    schema: { asset: asset("Asset to price") },
    call: (c, a) => c.prices.get(a.asset as string),
  },
  {
    name: "get_prices",
    description: "Get current USD prices for several assets in one call.",
    schema: {
      assets: z.array(z.string().min(1)).min(1).max(50).describe("Assets to price"),
    },
    call: (c, a) => c.prices.batch(a.assets as string[]),
  },
  {
    name: "convert_to_usd",
    description: "Convert an amount of an asset to USD using current price data.",
    schema: {
      asset: asset("Asset to convert"),
      amount: z.number().positive(),
    },
    call: (c, a) => c.prices.convertToUsd(a.asset as string, a.amount as number),
  },
  {
    name: "estimate_transfer_cost",
    description:
      "Estimate the fees, spread and slippage of a transfer between two currencies across the available payment routes, and the amount the recipient would receive.",
    schema: {
      source_currency: z.string().min(1).describe('Currency the sender pays in, e.g. "USDC"'),
      destination_currency: z
        .string()
        .min(1)
        .describe('Currency the recipient receives, e.g. "NGN"'),
      source_amount: z.number().positive().describe("Amount in the source currency"),
      destination_amount: z
        .number()
        .positive()
        .optional()
        .describe("Amount the recipient must receive, to report any shortfall"),
    },
    call: (c, a) =>
      c.costCalculator.estimate({
        source_currency: a.source_currency as string,
        destination_currency: a.destination_currency as string,
        source_amount: a.source_amount as number,
        destination_amount: a.destination_amount as number | undefined,
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
    schema: { id: z.string().min(1) },
    call: (c, a) => c.liquidityPools.get(a.id as string),
  },
  {
    name: "get_network_info",
    description: "Get the Stellar network this PayRaider instance reports on (name, RPC and Horizon URLs).",
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
    name: "list_governance_proposals",
    description: "List on-chain governance proposals.",
    schema: pagination,
    call: (c, a) => c.governance.listProposals(a),
  },
  {
    name: "get_governance_proposal",
    description: "Get detail for one governance proposal by ID.",
    schema: { id: z.string().min(1) },
    call: (c, a) => c.governance.getProposal(a.id as string),
  },
  {
    name: "list_alert_history",
    description:
      "List past triggered alerts for the signed-in user (read-only). Needs a user access token, not just an API key.",
    schema: pagination,
    requiresAuth: true,
    call: (c, a) => c.alerts.listHistory(a),
  },
  {
    name: "verify_asset",
    description: "Verify a Stellar asset (code + issuer) and return its verification status and risk signals.",
    schema: { asset_code: z.string().min(1), asset_issuer: z.string().min(1) },
    call: (c, a) => c.assetVerification.verify(a.asset_code as string, a.asset_issuer as string),
  },
  {
    name: "get_verified_asset",
    description: "Get a previously computed asset verification result.",
    schema: { asset_code: z.string().min(1), asset_issuer: z.string().min(1) },
    call: (c, a) => c.assetVerification.get(a.asset_code as string, a.asset_issuer as string),
  },
  {
    name: "list_verified_assets",
    description: "List assets that have been verified.",
    schema: pagination,
    call: (c, a) => c.assetVerification.list(a),
  },
];
