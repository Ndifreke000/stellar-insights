# MCP Server: Tool-Level Authorization and Cost Limits

**GitHub Issue**: [#2843](https://github.com/Ndifreke000/stellar-insights/issues/2843)  
**Estimated size**: L — 1–2 weeks for someone familiar with the MCP SDK and the PayRaider backend.

---

## Overview

The MCP server currently registers every tool in `READ_ONLY_TOOLS` unconditionally and enforces
no per-tool or per-session limits on how many times those tools may be called. An LLM agent that
loops — or a misconfigured one — can exhaust the operator's API quota silently, call tools the
user never intended to expose, or trigger backend rate-limiting that affects other users sharing
the same API key.

This document specifies:

1. A per-tool **allowlist and scope system** in `config.ts` so operators can restrict which tools
   are visible to agents.
2. A **per-session budget** (call count and cost units) with a structured error response when the
   budget is exhausted.
3. **Structured output schemas** for every tool that lacks one today.
4. The test harness and acceptance criteria.

Affected paths:

| File | Lines | What is missing |
|---|---|---|
| `sdk/mcp-server/src/tools.ts` | 267 | No per-tool scope metadata; no cost-unit annotation |
| `sdk/mcp-server/src/config.ts` | 81 | No allowlist, scope, or budget fields |
| `sdk/mcp-server/src/server.ts` | 89 | No allowlist filtering; no budget enforcement; no structured budget-error |

---

## Current State

```
sdk/mcp-server/src/
├── config.ts          ← loads env/argv; no tool allowlist or budget fields
├── server.ts          ← registers all READ_ONLY_TOOLS unconditionally
├── tools.ts           ← ToolDef array; no scope or cost-unit field
├── structured.ts      ← presentDefault, presentPreflight, presentPreflightBatch
├── resources.ts       ← MCP resource registrations
├── client.ts
├── sdk-types.ts
└── version.ts
```

`config.ts` builds a `ServerConfig` from environment variables. The only access-control field
today is `allowWrites: boolean`, which prevents the write-tool set from being registered but
applies to all tools in that set at once. `tools.ts` defines 22 tools; only
`preflight_payment` and `preflight_batch` have an `outputSchema`. `server.ts` iterates
`READ_ONLY_TOOLS` and calls `server.registerTool` for every entry with no filtering and no
budget tracking.

---

## Target Design

### Config schema: allowlist, scopes, and budgets

Add three new optional fields to `ServerConfig`. All are opt-in; omitting them keeps current
behaviour (all tools allowed, no budget).

```typescript
// sdk/mcp-server/src/config.ts

export type ToolScope = "read" | "preflight" | "pricing" | "governance" | "auth_required";

export interface ToolBudgetConfig {
  /** Maximum MCP tool calls per session. 0 = unlimited (default). */
  maxCalls: number;
  /** Maximum cost units per session. 0 = unlimited (default).
   *  Cost units are defined per tool in ToolDef.costUnits. */
  maxCostUnits: number;
  /** How the budget resets: "never" (default), "hourly", "daily". */
  resetPeriod: "never" | "hourly" | "daily";
}

export interface ServerConfig {
  apiKey?: string;
  baseUrl: string;
  network: "mainnet" | "testnet";
  transport: Transport;
  host: string;
  port: number;
  authToken?: string;
  allowedOrigins: string[];
  allowWrites: boolean;

  // --- new fields ---

  /**
   * Explicit list of tool names the agent may call.
   * Undefined or empty array = all tools allowed (current behaviour).
   */
  allowedTools?: string[];

  /**
   * Scope tags that must ALL appear on a tool for it to be callable.
   * Undefined or empty array = no scope restriction.
   */
  requiredScopes?: ToolScope[];

  /** Per-session call and cost budget. Undefined = no budget. */
  budget?: ToolBudgetConfig;
}
```

New environment variables that map to these fields:

| Variable | Type | Default | Description |
|---|---|---|---|
| `PAYRAIDER_MCP_ALLOWED_TOOLS` | comma-separated names | (all) | Explicit tool allowlist |
| `PAYRAIDER_MCP_REQUIRED_SCOPES` | comma-separated scopes | (none) | Scope filter |
| `PAYRAIDER_MCP_MAX_CALLS` | integer ≥ 0 | 0 (unlimited) | Call budget per session |
| `PAYRAIDER_MCP_MAX_COST_UNITS` | integer ≥ 0 | 0 (unlimited) | Cost-unit budget per session |
| `PAYRAIDER_MCP_BUDGET_RESET` | `never` / `hourly` / `daily` | `never` | Budget reset period |

### ToolDef: scope and cost-unit annotations

Extend `ToolDef` in `tools.ts` with two new fields:

```typescript
// sdk/mcp-server/src/tools.ts

export interface ToolDef {
  name: string;
  description: string;
  schema: z.ZodRawShape;
  requiresAuth?: boolean;
  call: (client: PayRaiderClient, args: Record<string, unknown>) => Promise<unknown>;
  outputSchema?: z.ZodRawShape;
  present?: (result: unknown, args: Record<string, unknown>) => Presented;

  // --- new fields ---

  /**
   * Scope tags describing this tool's access class.
   * Used when ServerConfig.requiredScopes is set.
   */
  scopes: ToolScope[];

  /**
   * Cost units consumed per successful call.
   * Counted against ServerConfig.budget.maxCostUnits.
   * Lightweight read tools cost 1; batch or expensive tools cost more.
   */
  costUnits: number;
}
```

Suggested scope and cost annotations for existing tools:

| Tool name | scopes | costUnits |
|---|---|---|
| `preflight_payment` | `["read", "preflight"]` | 1 |
| `preflight_batch` | `["read", "preflight"]` | 5 |
| `list_corridors` | `["read"]` | 1 |
| `get_corridor` | `["read"]` | 1 |
| `list_anchors` | `["read"]` | 1 |
| `get_anchor` | `["read"]` | 1 |
| `get_anchor_by_account` | `["read"]` | 1 |
| `get_price` | `["read", "pricing"]` | 1 |
| `get_prices` | `["read", "pricing"]` | 2 |
| `convert_to_usd` | `["read", "pricing"]` | 1 |
| `estimate_transfer_cost` | `["read", "pricing"]` | 2 |
| `list_liquidity_pools` | `["read"]` | 1 |
| `get_liquidity_pool` | `["read"]` | 1 |
| `get_network_info` | `["read"]` | 1 |
| `list_available_networks` | `["read"]` | 1 |
| `list_governance_proposals` | `["read", "governance"]` | 1 |
| `get_governance_proposal` | `["read", "governance"]` | 1 |
| `list_alert_history` | `["read", "auth_required"]` | 1 |
| `verify_asset` | `["read"]` | 2 |
| `get_verified_asset` | `["read"]` | 1 |
| `list_verified_assets` | `["read"]` | 1 |

### Budget enforcement flow

The budget is tracked in a `SessionBudget` object that `buildServer` creates once per server
instance (or once per HTTP session for the HTTP transport — see the security notes below).

```
agent calls tool
      │
      ▼
  Is tool in allowedTools (if set)?  ──No──► return TOOL_NOT_ALLOWED error
      │ Yes
      ▼
  Does tool satisfy requiredScopes?  ──No──► return TOOL_NOT_ALLOWED error
      │ Yes
      ▼
  budget.maxCalls > 0 AND
  session.calls >= budget.maxCalls?  ──Yes──► return BUDGET_EXHAUSTED error
      │ No
      ▼
  budget.maxCostUnits > 0 AND
  session.costUnits + tool.costUnits
        > budget.maxCostUnits?        ──Yes──► return BUDGET_EXHAUSTED error
      │ No
      ▼
  execute tool.call(...)
      │
      ▼
  session.calls += 1
  session.costUnits += tool.costUnits
      │
      ▼
  return result
```

### Structured output schemas for every tool

Every `ToolDef` without an `outputSchema` today must gain one. The existing
`PREFLIGHT_OUTPUT_SCHEMA` and `PREFLIGHT_BATCH_OUTPUT_SCHEMA` in `structured.ts` are the model.
Add schemas in `structured.ts` following the same pattern:

```typescript
// sdk/mcp-server/src/structured.ts (additions)

export const CORRIDOR_OUTPUT_SCHEMA = {
  id: z.string(),
  source_asset: z.string(),
  destination_asset: z.string(),
  success_rate: z.number(),
  health_score: z.number(),
  // ... full field list matching the backend response
} satisfies z.ZodRawShape;

export const ANCHOR_OUTPUT_SCHEMA = { /* ... */ } satisfies z.ZodRawShape;
export const PRICE_OUTPUT_SCHEMA   = { /* ... */ } satisfies z.ZodRawShape;
// one per tool
```

Each tool entry in `READ_ONLY_TOOLS` then sets `outputSchema` to the corresponding constant.

### Structured error response shape

Both `TOOL_NOT_ALLOWED` and `BUDGET_EXHAUSTED` must return a structured MCP error so agents can
detect them programmatically without parsing free text.

```typescript
// sdk/mcp-server/src/server.ts

export type McpErrorCode = "TOOL_NOT_ALLOWED" | "BUDGET_EXHAUSTED" | "API_ERROR";

export interface McpStructuredError {
  error: McpErrorCode;
  message: string;
  /** Present for BUDGET_EXHAUSTED only. */
  budget?: {
    maxCalls: number;
    usedCalls: number;
    maxCostUnits: number;
    usedCostUnits: number;
    resetPeriod: string;
  };
}
```

Wire response example for a budget-exhausted event:

```json
{
  "content": [
    {
      "type": "text",
      "text": "Budget exhausted: session call limit of 50 reached."
    }
  ],
  "structuredContent": {
    "error": "BUDGET_EXHAUSTED",
    "message": "Session call limit of 50 reached.",
    "budget": {
      "maxCalls": 50,
      "usedCalls": 50,
      "maxCostUnits": 0,
      "usedCostUnits": 32,
      "resetPeriod": "never"
    }
  },
  "isError": true
}
```

Wire response example for a disallowed tool:

```json
{
  "content": [
    { "type": "text", "text": "Tool not allowed: list_alert_history is not in the configured allowlist." }
  ],
  "structuredContent": {
    "error": "TOOL_NOT_ALLOWED",
    "message": "list_alert_history is not in the configured allowlist."
  },
  "isError": true
}
```

---

## Implementation Steps

### 1. Extend `ToolDef` with `scopes` and `costUnits`

In `sdk/mcp-server/src/tools.ts`, add the two fields to the `ToolDef` interface and annotate
every entry in `READ_ONLY_TOOLS` using the table in the target design above. Tools must not
compile without both fields — make them required (no `?`), so TypeScript will error on any tool
that omits them.

### 2. Extend `ServerConfig` and `loadConfig`

In `sdk/mcp-server/src/config.ts`:

- Add `allowedTools`, `requiredScopes`, and `budget` to `ServerConfig`.
- In `loadConfig`, parse the new environment variables:

```typescript
function parseCommaSeparated(raw: string | undefined): string[] {
  if (!raw) return [];
  return raw.split(",").map((s) => s.trim()).filter(Boolean);
}

function parseBudget(env: Record<string, string | undefined>): ToolBudgetConfig | undefined {
  const maxCalls     = parseInt(env.PAYRAIDER_MCP_MAX_CALLS ?? "0", 10);
  const maxCostUnits = parseInt(env.PAYRAIDER_MCP_MAX_COST_UNITS ?? "0", 10);
  if (maxCalls === 0 && maxCostUnits === 0) return undefined;
  const resetPeriod = (env.PAYRAIDER_MCP_BUDGET_RESET ?? "never") as ToolBudgetConfig["resetPeriod"];
  if (!["never", "hourly", "daily"].includes(resetPeriod)) {
    throw new ConfigError(`PAYRAIDER_MCP_BUDGET_RESET must be "never", "hourly", or "daily"`);
  }
  return { maxCalls, maxCostUnits, resetPeriod };
}

// inside loadConfig():
allowedTools:   parseCommaSeparated(env.PAYRAIDER_MCP_ALLOWED_TOOLS),
requiredScopes: parseCommaSeparated(env.PAYRAIDER_MCP_REQUIRED_SCOPES) as ToolScope[],
budget:         parseBudget(env),
```

### 3. Add output schemas for every tool

In `sdk/mcp-server/src/structured.ts`, define a `z.ZodRawShape` constant for every tool that
lacks one (all tools except `preflight_payment` and `preflight_batch`). Shape each schema to
match the actual backend response type, referencing the `@payraider/sdk` TypeScript types where
they exist.

Then set `outputSchema` on every entry in `READ_ONLY_TOOLS`.

### 4. Implement `SessionBudget` and allowlist filtering in `server.ts`

```typescript
// sdk/mcp-server/src/server.ts

interface SessionBudget {
  calls: number;
  costUnits: number;
  resetAt: number | null; // epoch ms, null for "never"
}

function makeSessionBudget(cfg: ToolBudgetConfig | undefined): SessionBudget {
  return { calls: 0, costUnits: 0, resetAt: null };
}

function checkAndResetBudget(budget: SessionBudget, cfg: ToolBudgetConfig): void {
  if (cfg.resetPeriod === "never" || budget.resetAt === null) return;
  if (Date.now() >= budget.resetAt) {
    budget.calls = 0;
    budget.costUnits = 0;
    budget.resetAt = nextResetAt(cfg.resetPeriod);
  }
}
```

In `buildServer`, filter `READ_ONLY_TOOLS` before the registration loop:

```typescript
const allowedSet  = new Set(config.allowedTools ?? []);
const requiredScopes: ToolScope[] = config.requiredScopes ?? [];

const visibleTools = READ_ONLY_TOOLS.filter((tool) => {
  if (allowedSet.size > 0 && !allowedSet.has(tool.name)) return false;
  if (requiredScopes.length > 0 && !requiredScopes.every((s) => tool.scopes.includes(s))) return false;
  return true;
});
```

Inside the tool handler, enforce the budget before calling `tool.call`:

```typescript
const session = makeSessionBudget(config.budget);

// inside the per-tool handler:
if (config.budget) {
  checkAndResetBudget(session, config.budget);
  if (config.budget.maxCalls > 0 && session.calls >= config.budget.maxCalls) {
    return budgetExhaustedError("call", config.budget, session);
  }
  if (
    config.budget.maxCostUnits > 0 &&
    session.costUnits + tool.costUnits > config.budget.maxCostUnits
  ) {
    return budgetExhaustedError("cost", config.budget, session);
  }
}
// ... existing tool.call(...) logic ...
// on success:
if (config.budget) {
  session.calls      += 1;
  session.costUnits  += tool.costUnits;
}
```

### 5. Update `.env.example`

```ini
# Tool-level authorization (comma-separated names; empty = all tools)
PAYRAIDER_MCP_ALLOWED_TOOLS=

# Scope filter (comma-separated: read, preflight, pricing, governance, auth_required)
PAYRAIDER_MCP_REQUIRED_SCOPES=

# Per-session budget (0 = unlimited)
PAYRAIDER_MCP_MAX_CALLS=0
PAYRAIDER_MCP_MAX_COST_UNITS=0
PAYRAIDER_MCP_BUDGET_RESET=never
```

### 6. Update `sdk/mcp-server/README.md`

Add a "Tool authorization and budgets" section documenting the new environment variables, the
scope system, the structured error codes, and a minimal configuration example for a restricted
agent that may only run preflight checks with a 100-call session limit.

---

## Acceptance Criteria

- [ ] `PAYRAIDER_MCP_ALLOWED_TOOLS=preflight_payment,preflight_batch` causes all other tools to
  return a `TOOL_NOT_ALLOWED` structured error when called; the disallowed tools do not appear in
  the tool listing sent to the client.
- [ ] `PAYRAIDER_MCP_REQUIRED_SCOPES=pricing` causes only `get_price`, `get_prices`,
  `convert_to_usd`, and `estimate_transfer_cost` to be visible.
- [ ] A session that reaches `PAYRAIDER_MCP_MAX_CALLS` returns `BUDGET_EXHAUSTED` with the
  `budget` block populated; calls beyond the limit are not forwarded to the backend.
- [ ] A session that would exceed `PAYRAIDER_MCP_MAX_COST_UNITS` returns `BUDGET_EXHAUSTED`
  before `tool.call` is invoked.
- [ ] Every tool has an `outputSchema`; `tsc --noEmit` passes with no errors.
- [ ] CI is green (lint, type-check, existing tests, new tests).
- [ ] No new `TODO` comments without a linked GitHub issue.
- [ ] `sdk/mcp-server/README.md` documents the new environment variables and error codes.

---

## Test Plan

Tests live in `sdk/mcp-server/src/__tests__/` and use an in-process MCP client harness that
communicates over the stdio transport with a mock `PayRaiderClient`.

### Disallowed tool is rejected

```typescript
// sdk/mcp-server/src/__tests__/authorization.test.ts

it("returns TOOL_NOT_ALLOWED when tool is not in allowedTools", async () => {
  const server = buildServer({
    ...baseConfig,
    allowedTools: ["preflight_payment"],
  });
  const result = await callTool(server, "get_price", { asset: "XLM:native" });

  expect(result.isError).toBe(true);
  expect(result.structuredContent).toMatchObject({
    error: "TOOL_NOT_ALLOWED",
    message: expect.stringContaining("get_price"),
  });
});

it("does not register disallowed tools in the tool listing", async () => {
  const server = buildServer({
    ...baseConfig,
    allowedTools: ["preflight_payment"],
  });
  const tools = await listTools(server);
  expect(tools.map((t) => t.name)).toEqual(["preflight_payment"]);
});
```

### Budget exhaustion returns a structured error

```typescript
// sdk/mcp-server/src/__tests__/budget.test.ts

it("returns BUDGET_EXHAUSTED after maxCalls is reached", async () => {
  const server = buildServer({
    ...baseConfig,
    budget: { maxCalls: 2, maxCostUnits: 0, resetPeriod: "never" },
  });

  await callTool(server, "get_network_info", {});  // call 1 — succeeds
  await callTool(server, "get_network_info", {});  // call 2 — succeeds
  const result = await callTool(server, "get_network_info", {}); // call 3

  expect(result.isError).toBe(true);
  expect(result.structuredContent).toMatchObject({
    error: "BUDGET_EXHAUSTED",
    budget: { maxCalls: 2, usedCalls: 2 },
  });
});

it("returns BUDGET_EXHAUSTED when costUnits would exceed maxCostUnits", async () => {
  // preflight_batch costs 5 units; set budget to 4
  const server = buildServer({
    ...baseConfig,
    budget: { maxCalls: 0, maxCostUnits: 4, resetPeriod: "never" },
  });

  const result = await callTool(server, "preflight_batch", { payments: [samplePayment()] });

  expect(result.isError).toBe(true);
  expect(result.structuredContent).toMatchObject({
    error: "BUDGET_EXHAUSTED",
    budget: { maxCostUnits: 4, usedCostUnits: 0 },
  });
});
```

### Scope filtering

```typescript
// sdk/mcp-server/src/__tests__/scopes.test.ts

it("only registers tools whose scopes include all requiredScopes", async () => {
  const server = buildServer({
    ...baseConfig,
    requiredScopes: ["pricing"],
  });
  const tools = await listTools(server);
  const names = tools.map((t) => t.name).sort();
  expect(names).toEqual([
    "convert_to_usd",
    "estimate_transfer_cost",
    "get_price",
    "get_prices",
  ]);
});
```

### No budget when config is absent

```typescript
it("forwards calls to the backend without restriction when budget is undefined", async () => {
  const server = buildServer({ ...baseConfig }); // no budget field
  const mockClient = server.__testClient;

  for (let i = 0; i < 200; i++) {
    const r = await callTool(server, "get_network_info", {});
    expect(r.isError).toBeFalsy();
  }
  expect(mockClient.network.info).toHaveBeenCalledTimes(200);
});
```

---

## Security Design Notes

### Principle of least privilege

`allowedTools` and `requiredScopes` are opt-in; the defaults keep current behaviour. Operators
deploying to production are expected to set an explicit allowlist containing only the tools their
agent workflow requires. The scope system provides a coarser secondary filter, useful when the
tool list may change and the operator wants to restrict by capability class rather than by name.

Tool names in `PAYRAIDER_MCP_ALLOWED_TOOLS` are matched exactly (case-sensitive) against
`tool.name`. Unknown names in the allowlist are silently ignored rather than treated as an
error, to tolerate configuration that pre-lists a tool before it ships, but a warning is logged
at startup for each unrecognised name.

### Budget reset semantics

The `resetPeriod` controls when `session.calls` and `session.costUnits` reset to zero:

- **`never`** (default): the budget is a hard lifetime cap for the server instance. This is the
  safe default because it prevents unbounded cost accumulation over a long-running agent session.
- **`hourly` / `daily`**: the budget resets at the next clock-aligned boundary (top of the hour
  or midnight UTC). These are appropriate when the operator sets a daily API key quota and wants
  to distribute it across time windows.

For the HTTP transport, each HTTP connection is a separate session and therefore gets its own
`SessionBudget` instance. For stdio, the single process is one session. Document this distinction
in the README.

### Error response format

Budget and authorization errors set `isError: true` and populate `structuredContent` with an
`McpStructuredError` object. Agents can branch on `structuredContent.error` without parsing the
human-readable `content[0].text`. The `message` field is English prose and must not contain
internal implementation details (stack traces, file paths, internal error codes).

The `budget` block in a `BUDGET_EXHAUSTED` response reports the current counters so the agent
can communicate the situation to the user accurately ("session limit of 50 calls reached, 32
cost units used").

---

## References

- [Model Context Protocol specification](https://spec.modelcontextprotocol.io/)
- [sdk/mcp-server/src/tools.ts](../sdk/mcp-server/src/tools.ts)
- [sdk/mcp-server/src/config.ts](../sdk/mcp-server/src/config.ts)
- [sdk/mcp-server/src/server.ts](../sdk/mcp-server/src/server.ts)
- [sdk/mcp-server/src/structured.ts](../sdk/mcp-server/src/structured.ts)
- [SECURITY.md](../SECURITY.md)
- [DEPENDENCIES.md](./DEPENDENCIES.md)
