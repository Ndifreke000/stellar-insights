import { z } from "zod";
import type { ToolDef } from "./tools.js";

/**
 * Mutating tools - only registered when STELLAR_INSIGHTS_MCP_ALLOW_WRITES=true
 * (see index.ts). Excluded even then: anchor/corridor creation (not exposed
 * by @stellar-insights/sdk's resources at all) and auth.login/refresh/logout
 * (the server already authenticates with one API key - there is no reason
 * for an agent to manage a separate credential session).
 */
export const WRITE_TOOLS: ToolDef[] = [
  {
    name: "submit_transaction_envelope",
    description:
      "Submit a signed Stellar transaction envelope (XDR) for execution. Moves real funds/state - " +
      "the envelope must already be fully signed before calling this.",
    schema: { envelope_xdr: z.string().describe("Base64-encoded, fully-signed transaction envelope XDR") },
    call: (c, a) => c.transactions.create({ envelope_xdr: a.envelope_xdr as string }),
  },
  {
    name: "resubmit_transaction",
    description: "Resubmit a previously created transaction by ID (e.g. after a fee-bump or retry).",
    schema: { id: z.string() },
    call: (c, a) => c.transactions.submit(a.id as string),
  },
  {
    name: "create_governance_proposal",
    description: "Create a new on-chain governance proposal.",
    schema: { title: z.string(), description: z.string() },
    call: (c, a) => c.governance.createProposal({ title: a.title as string, description: a.description as string }),
  },
  {
    name: "vote_on_governance_proposal",
    description: "Cast a vote (support or oppose) on a governance proposal.",
    schema: { id: z.string(), support: z.boolean() },
    call: (c, a) => c.governance.vote(a.id as string, a.support as boolean),
  },
  {
    name: "create_alert_rule",
    description: "Create a new alert rule (e.g. corridor success-rate below a threshold).",
    schema: {
      name: z.string(),
      condition: z.string().describe("Condition expression, e.g. 'success_rate < threshold'"),
      threshold: z.number(),
    },
    call: (c, a) =>
      c.alerts.createRule({
        name: a.name as string,
        condition: a.condition as string,
        threshold: a.threshold as number,
      }),
  },
  {
    name: "update_alert_rule",
    description: "Update an existing alert rule's name, condition, and/or threshold.",
    schema: {
      id: z.string(),
      name: z.string().optional(),
      condition: z.string().optional(),
      threshold: z.number().optional(),
    },
    call: (c, a) => {
      const { id, ...rest } = a as { id: string; name?: string; condition?: string; threshold?: number };
      return c.alerts.updateRule(id, rest);
    },
  },
  {
    name: "delete_alert_rule",
    description: "Delete an alert rule by ID.",
    schema: { id: z.string() },
    call: (c, a) => c.alerts.deleteRule(a.id as string),
  },
  {
    name: "create_webhook",
    description: "Register a new webhook endpoint for event notifications.",
    schema: { url: z.string().url(), events: z.array(z.string()).describe("Event types to subscribe to") },
    call: (c, a) => c.webhooks.create({ url: a.url as string, events: a.events as string[] }),
  },
  {
    name: "delete_webhook",
    description: "Delete a webhook by ID.",
    schema: { id: z.string() },
    call: (c, a) => c.webhooks.delete(a.id as string),
  },
  {
    name: "test_webhook",
    description: "Send a test event to a webhook to verify it's reachable and configured correctly.",
    schema: { id: z.string() },
    call: (c, a) => c.webhooks.test(a.id as string),
  },
  {
    name: "create_api_key",
    description: "Create a new API key. The returned key is only ever shown once - store it immediately.",
    schema: { name: z.string().describe("Human-readable label for the key") },
    call: (c, a) => c.apiKeys.create({ name: a.name as string }),
  },
  {
    name: "rotate_api_key",
    description: "Rotate an API key, invalidating the old value and returning a new one.",
    schema: { id: z.string() },
    call: (c, a) => c.apiKeys.rotate(a.id as string),
  },
  {
    name: "revoke_api_key",
    description: "Revoke (permanently disable) an API key by ID.",
    schema: { id: z.string() },
    call: (c, a) => c.apiKeys.revoke(a.id as string),
  },
];
