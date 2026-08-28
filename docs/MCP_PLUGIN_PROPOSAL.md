# Stellar Insights MCP Plugin — Business Model, Architecture & Work Breakdown

Status: proposal. Scope: expose Stellar Insights corridor/anchor/network analytics
to AI agents (Claude, ChatGPT, and other MCP-compatible clients) as a Model
Context Protocol (MCP) server, distributed as an installable plugin.

---

## 1. Why this, and why now

Stellar Insights already has everything an MCP server needs except the adapter
layer itself: a stable versioned REST API (`/api/v1/*`), a GraphQL endpoint, an
existing API-key auth/tiering system (anonymous / authenticated / premium), and
a typed TypeScript SDK (`sdk/typescript/src/resources.ts`) wrapping every
domain resource (corridors, anchors, network, liquidity pools, governance,
ML anomaly detection, cost calculator, prices). An MCP server here is a thin
adapter over that SDK, not new backend product work.

Compared with Dune — the closest general-purpose analytics platform — Dune's
extensibility comes from opening a SQL query surface to the community. Stellar
Insights' schema is fixed and vertical, but that is actually an advantage for
an *agent* tool surface: fixed, named, well-typed operations are exactly what
MCP tools want, versus an agent having to write ad-hoc SQL. And Stellar
Insights has one feature no generic chain-agnostic analytics product has: the
`stellar_insights` Soroban contract anchors a SHA-256 hash of each analytics
snapshot on-chain, so an agent can be given a tool that *proves* a metric
hasn't been tampered with — not just asserts it.

---

## 2. Business model

### 2.1 Value proposition
- **For anchor operators / fintechs building on Stellar:** let their own
  support bots, internal ops tools, and AI copilots answer "is corridor X
  healthy right now" or "why did anchor Y's SEP-24 flow degrade" without
  hand-building an integration.
- **For AI agent builders / Stellar ecosystem devs:** a drop-in tool source
  for any Claude/ChatGPT-based agent that needs live Stellar payment-network
  ground truth instead of stale training data.
- **For treasury / compliance teams:** the `verify_snapshot` tool gives an
  agent a way to cryptographically confirm a reported metric matches the
  on-chain-anchored value — relevant for anyone who has to attest to numbers.

### 2.2 Target segments
1. Stellar anchors and payment fintechs (SEP-6/24/31 operators) — highest
   willingness to pay, already paying for API access today.
2. Independent Stellar ecosystem developers / DAOs — free-tier, volume/community play.
3. AI agent platform builders (outside the Stellar ecosystem) who want a
   "financial network health" tool source — discovery via MCP registries.

### 2.3 Revenue model
Reuses the rate-limit tiers that already exist in the backend
(`rate_limit.rs`, `api_key_rate_limit_middleware`) — no new billing system:

| Tier | Tools exposed | Rate limit | Price |
|---|---|---|---|
| Free / community | Read-only (corridors, anchors, network, prices) | Existing anonymous tier limits | $0 — top-of-funnel, registry discoverability |
| Pro (existing API key) | + analytics, ML anomaly detection, cost calculator, `verify_snapshot` | Existing authenticated tier | Same price as current API key plans |
| Enterprise | + governance tooling, higher burst limits, dedicated instance option | Custom | Custom — sold alongside existing enterprise API contracts |

Write-scoped tools (create anchor/corridor, submit governance vote) are a
**separate, explicitly-opted-in SKU**, not bundled by default — see §3.4.

### 2.4 Distribution
- npm package (`npx @stellar-insights/mcp-server`) — zero-install trial.
- Listing in public MCP registries (mcp.so, Anthropic's directory) — this
  listing *is* the plugin distribution channel; no new storefront needed.
- Claude Desktop / Claude Code config snippet in the SDK README, next to the
  existing TS/Python SDK usage examples.

### 2.5 Cost structure
Marginal cost is near-zero: the MCP server adds a stateless adapter process
in front of infrastructure that already exists (Redis cache, SQLite, RPC
ingestion, rate limiter). The only new operational cost is monitoring/support
for the adapter itself.

### 2.6 Moat
The `verify_snapshot` tool (on-chain hash verification via the
`stellar_insights` Soroban contract) is not something a chain-agnostic tool
like Dune can offer for Stellar specifically without replicating this
project's contract layer. That is the durable differentiator to lead with in
any registry listing or marketing.

---

## 3. Architecture

### 3.1 System diagram

```mermaid
flowchart LR
    subgraph Agent Side
        A[Claude / ChatGPT / MCP client]
    end

    subgraph "MCP Server (new — thin adapter)"
        M[stellar-insights-mcp-server<br/>Node/TS, stdio + streamable HTTP]
        T[Tool layer<br/>Zod-typed tool defs]
        R[Resource layer<br/>stellar-insights://corridor/id etc.]
    end

    subgraph Existing, unchanged
        SDK[sdk/typescript resources<br/>HttpClient]
        API[Backend REST /api/v1 + GraphQL<br/>Rust / Axum]
        AUTH[Existing API-key + JWT tiering]
        DB[(SQLite + Redis cache)]
        RPC[Stellar Horizon / Soroban RPC]
        SC[Soroban contract:<br/>stellar_insights snapshot hash]
    end

    A <-->|MCP tool calls| M
    M --> T --> SDK
    M --> R --> SDK
    SDK -->|X-Api-Key| AUTH
    AUTH --> API
    API --> DB
    API --> RPC
    T -->|verify_snapshot| SC
```

### 3.2 Auth (reused, not rebuilt)
The MCP server holds one `STELLAR_INSIGHTS_API_KEY` (env var, operator-supplied)
and constructs a single `StellarInsightsClient` per process — identical to how
any other SDK consumer authenticates today. Tier (free/pro/enterprise) is
resolved server-side by the existing key, so quota enforcement needs no new
code.

### 3.3 Tool catalog (v1, all read-only)

| Tool | Backend call | Notes |
|---|---|---|
| `list_corridors` / `get_corridor` / `compare_corridors` | `CorridorsResource` | Core dataset |
| `list_anchors` / `get_anchor` / `get_anchor_assets` | `AnchorsResource` | |
| `get_network_stats` | `NetworkResource` | |
| `get_analytics` | analytics_dashboard routes | period: 24h/7d/30d/90d |
| `get_liquidity_pools` | `LiquidityPoolsResource` | |
| `get_price` / `list_prices` | `PricesResource` | |
| `estimate_transfer_cost` | `CostCalculatorResource` | |
| `predict_anomaly` / `get_ml_status` | `MlResource` | |
| `get_governance_proposals` | `GovernanceResource` | read-only |
| `get_alert_history` | alerts routes | read-only; rule *creation* excluded |
| `verify_snapshot` | Soroban `stellar_insights` contract + backend snapshot | the differentiator tool |

### 3.4 Security boundary
Default tool surface is **read-only**. `create_anchor`, `create_corridor`,
governance vote submission, and webhook management are excluded from the
default build entirely and only available behind an explicit
`--allow-writes` operator flag — an agent acting on injected or adversarial
content must not be able to mutate anchor registries or cast governance
votes.

### 3.5 MCP resources (context, not just tools)
`stellar-insights://corridor/{id}` and `stellar-insights://anchor/{id}` are
exposed as MCP resource URIs so a user in Claude can attach "this corridor" to
context directly, mirroring the existing corridor/anchor detail pages in the
frontend.

---

## 4. Work breakdown structure

Note: Phase 0 is a **hard prerequisite** discovered during debugging — the
backend currently does not compile from a clean clone (see PR #2035 for the
`Cargo.toml` portion, already fixed). The MCP server cannot be tested against
a live backend until Phase 0 is complete.

| ID | Task | Deliverable | Depends on | Est. effort |
|---|---|---|---|---|
| 0.1 | Fix `Cargo.toml` duplicate keys | Backend manifest parses | — | Done (PR #2035) |
| 0.2 | Fix redis-rs API mismatch (`cache.rs`, `redis_caching_layer.rs`, `rate_limiting_advanced.rs`, `distributed_lock.rs`, `websocket.rs`, `api/gdpr.rs`, `ingestion/*`, `rate_limit.rs`) | Backend compiles against redis 1.3 | 0.1 | 2–4 days |
| 0.3 | Fix hmac/sha2/digest version conflict (`webhooks/mod.rs`, `auth/sep10_simple.rs`, `request_signing_middleware.rs`) | Webhook signing + SEP-10 compile | 0.1 | 0.5–1 day |
| 0.4 | Fix frontend `package-lock.json` drift (`@vitejs/plugin-react` ^4 vs ^6) | `npm install` succeeds with no flags | — | 0.5 day |
| 1.1 | Scaffold `@stellar-insights/mcp-server` package on `@modelcontextprotocol/sdk`, import `sdk/typescript` resources | Empty server boots, stdio transport | 0.2, 0.3 | 0.5 day |
| 1.2 | Add streamable-HTTP transport option | Server runnable as hosted endpoint, not just local stdio | 1.1 | 0.5 day |
| 2.1 | Implement read-only tool set (§3.3) with Zod schemas + descriptions | 11 working tools | 1.1 | 2–3 days |
| 2.2 | Implement `verify_snapshot` (Soroban contract read + hash compare) | Differentiator tool working end-to-end | 2.1 | 1–2 days |
| 2.3 | Implement MCP resource URIs (`stellar-insights://corridor/{id}`, `.../anchor/{id}`) | Agents can attach entities as context | 2.1 | 0.5 day |
| 3.1 | API-key auth wiring + tier-aware error messages | Rate-limit errors surface cleanly to the agent | 1.1 | 0.5 day |
| 3.2 | `--allow-writes` gate + write-tool implementations (kept off by default) | Opt-in write tools, off in default build | 2.1 | 1 day |
| 4.1 | Integration tests against a running backend (real testnet data) | CI-runnable test suite | 0.2, 0.3 | 1–2 days |
| 4.2 | Manual test in Claude Desktop / Claude Code | Verified working end-to-end in a real client | 2.1, 2.2 | 0.5 day |
| 5.1 | npm publish (`@stellar-insights/mcp-server`), README + config snippets | Installable via `npx` | 4.2 | 0.5 day |
| 5.2 | Submit to MCP registries (mcp.so, Anthropic directory) | Public discoverability | 5.1 | 0.5 day |
| 6.1 | Docs: add MCP section to `sdk/` README, cross-link from main README | Documented alongside existing SDKs | 5.1 | 0.5 day |

**Total estimated effort:** ~14–19 days, of which ~3–5.5 days (Phase 0) is
fixing pre-existing backend breakage unrelated to the plugin itself, and
~11–13.5 days is the MCP server build proper.
