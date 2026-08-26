"use client";

/**
 * Issue #2131 — Public API Documentation Portal
 *
 * Interactive API docs with endpoint browser, code samples, and links to the
 * Playground and Examples sub-pages.
 */

import React, { useState, useMemo } from "react";
import Link from "next/link";
import {
  Search,
  BookOpen,
  Code2,
  FlaskConical,
  ChevronDown,
  ChevronRight,
  ExternalLink,
  Copy,
  Check,
  Zap,
  Shield,
  Activity,
  BarChart3,
  Globe,
} from "lucide-react";

// ---------------------------------------------------------------------------
// API endpoint catalogue
// ---------------------------------------------------------------------------

const METHOD_COLORS: Record<string, string> = {
  GET: "bg-emerald-500/10 text-emerald-400 border border-emerald-500/30",
  POST: "bg-blue-500/10 text-blue-400 border border-blue-500/30",
  PUT: "bg-amber-500/10 text-amber-400 border border-amber-500/30",
  DELETE: "bg-red-500/10 text-red-400 border border-red-500/30",
  PATCH: "bg-purple-500/10 text-purple-400 border border-purple-500/30",
};

interface ApiParam {
  name: string;
  type: string;
  required: boolean;
  description: string;
}

interface ApiEndpoint {
  method: "GET" | "POST" | "PUT" | "DELETE" | "PATCH";
  path: string;
  summary: string;
  description: string;
  category: string;
  params?: ApiParam[];
  example: {
    request?: string;
    response: string;
  };
  auth?: boolean;
}

const API_ENDPOINTS: ApiEndpoint[] = [
  // ── Analytics ──────────────────────────────────────────────────────────────
  {
    method: "GET",
    path: "/api/v1/analytics/corridors",
    summary: "List payment corridors",
    description:
      "Returns a paginated list of payment corridors with reliability scores, average settlement times, and volume data.",
    category: "Analytics",
    params: [
      { name: "limit", type: "integer", required: false, description: "Max results per page (default 20, max 100)" },
      { name: "offset", type: "integer", required: false, description: "Pagination offset" },
      { name: "sort", type: "string", required: false, description: "Sort field: reliability | volume | latency" },
    ],
    example: {
      response: JSON.stringify(
        {
          corridors: [
            { id: "USD-PHP", reliability: 0.98, avg_latency_ms: 4200, volume_24h: 1250000 },
            { id: "EUR-MXN", reliability: 0.95, avg_latency_ms: 5100, volume_24h: 840000 },
          ],
          total: 42,
          offset: 0,
        },
        null,
        2
      ),
    },
  },
  {
    method: "GET",
    path: "/api/v1/analytics/corridors/:id",
    summary: "Get corridor details",
    description: "Returns detailed analytics for a single corridor including historical success rates and fee data.",
    category: "Analytics",
    params: [
      { name: "id", type: "string", required: true, description: "Corridor identifier e.g. USD-PHP" },
    ],
    example: {
      response: JSON.stringify(
        {
          id: "USD-PHP",
          reliability: 0.98,
          avg_latency_ms: 4200,
          median_fee_pct: 0.12,
          volume_7d: 8750000,
          success_history: [0.97, 0.98, 0.99, 0.98],
        },
        null,
        2
      ),
    },
  },
  {
    method: "GET",
    path: "/api/v1/analytics/snapshots/:epoch",
    summary: "Get analytics snapshot",
    description:
      "Retrieves the on-chain analytics snapshot hash for a given epoch. Use this to verify data integrity against the Stellar Insights contract.",
    category: "Analytics",
    params: [
      { name: "epoch", type: "integer", required: true, description: "Epoch number (positive integer)" },
    ],
    example: {
      response: JSON.stringify(
        {
          epoch: 142,
          hash: "a3f2c1d4e5b6...",
          timestamp: 1722000000,
          verified: true,
        },
        null,
        2
      ),
    },
  },
  // ── Network ─────────────────────────────────────────────────────────────────
  {
    method: "GET",
    path: "/api/v1/network/health",
    summary: "Network health status",
    description: "Returns current Stellar network health indicators including ledger close times and validator consensus.",
    category: "Network",
    example: {
      response: JSON.stringify(
        {
          status: "healthy",
          ledger_sequence: 51234567,
          close_time_ms: 5100,
          quorum_health: 1.0,
        },
        null,
        2
      ),
    },
  },
  {
    method: "GET",
    path: "/api/v1/network/graph",
    summary: "Payment network graph",
    description: "Returns nodes and edges of the Stellar payment network for visualisation.",
    category: "Network",
    params: [
      { name: "depth", type: "integer", required: false, description: "Graph traversal depth (1-3, default 2)" },
      { name: "anchor", type: "string", required: false, description: "Anchor address to start from" },
    ],
    example: {
      response: JSON.stringify(
        {
          nodes: [{ id: "G...", type: "anchor", label: "StellarX" }],
          edges: [{ source: "G...", target: "G...", weight: 0.85 }],
        },
        null,
        2
      ),
    },
  },
  // ── Payments ────────────────────────────────────────────────────────────────
  {
    method: "POST",
    path: "/api/v1/payments/route",
    summary: "Compute optimal payment route",
    description:
      "Given a source asset, destination asset, and amount, returns the lowest-cost route with predicted success probability.",
    category: "Payments",
    auth: true,
    params: [
      { name: "source_asset", type: "string", required: true, description: "Source asset code e.g. USD" },
      { name: "dest_asset", type: "string", required: true, description: "Destination asset code e.g. PHP" },
      { name: "amount", type: "number", required: true, description: "Amount in source asset" },
    ],
    example: {
      request: JSON.stringify(
        { source_asset: "USD", dest_asset: "PHP", amount: 100 },
        null,
        2
      ),
      response: JSON.stringify(
        {
          route: ["USD", "XLM", "PHP"],
          estimated_fee_pct: 0.11,
          success_probability: 0.97,
          estimated_time_ms: 4800,
        },
        null,
        2
      ),
    },
  },
  {
    method: "GET",
    path: "/api/v1/payments/history",
    summary: "Payment history",
    description: "Returns paginated payment history for an account.",
    category: "Payments",
    auth: true,
    params: [
      { name: "account", type: "string", required: true, description: "Stellar account address (G...)" },
      { name: "limit", type: "integer", required: false, description: "Results per page (default 20)" },
      { name: "cursor", type: "string", required: false, description: "Paging cursor for next page" },
    ],
    example: {
      response: JSON.stringify(
        {
          payments: [
            { id: "...", amount: "100", asset: "USD", status: "success", latency_ms: 4200 },
          ],
          next_cursor: "abc123",
        },
        null,
        2
      ),
    },
  },
  // ── Metrics ─────────────────────────────────────────────────────────────────
  {
    method: "GET",
    path: "/api/v1/metrics/dashboard",
    summary: "Dashboard KPI metrics",
    description: "Returns key performance indicators shown on the main dashboard.",
    category: "Metrics",
    example: {
      response: JSON.stringify(
        {
          total_payments_24h: 142000,
          success_rate_24h: 0.972,
          avg_fee_pct: 0.13,
          active_corridors: 38,
        },
        null,
        2
      ),
    },
  },
  {
    method: "GET",
    path: "/api/v1/metrics/latency",
    summary: "Settlement latency percentiles",
    description: "Returns p50 / p95 / p99 settlement latency in milliseconds for a given corridor or all corridors.",
    category: "Metrics",
    params: [
      { name: "corridor", type: "string", required: false, description: "Corridor ID; omit for global stats" },
      { name: "range", type: "string", required: false, description: "Time range: 1h | 24h | 7d | 30d (default 24h)" },
    ],
    example: {
      response: JSON.stringify(
        { p50_ms: 4100, p95_ms: 8300, p99_ms: 14200, corridor: "all", range: "24h" },
        null,
        2
      ),
    },
  },
  // ── Alerts ──────────────────────────────────────────────────────────────────
  {
    method: "GET",
    path: "/api/v1/alerts",
    summary: "List alert rules",
    description: "Returns all configured alert rules for the authenticated user.",
    category: "Alerts",
    auth: true,
    example: {
      response: JSON.stringify(
        {
          alerts: [
            { id: "1", name: "High latency", condition: "latency_ms > 10000", active: true },
          ],
        },
        null,
        2
      ),
    },
  },
  {
    method: "POST",
    path: "/api/v1/alerts",
    summary: "Create alert rule",
    description: "Creates a new threshold-based alert rule.",
    category: "Alerts",
    auth: true,
    params: [
      { name: "name", type: "string", required: true, description: "Human-readable alert name" },
      { name: "condition", type: "string", required: true, description: "Alert condition expression" },
      { name: "channels", type: "array", required: true, description: "Notification channels: email | webhook | push" },
    ],
    example: {
      request: JSON.stringify(
        { name: "Low reliability", condition: "reliability < 0.90", channels: ["email"] },
        null,
        2
      ),
      response: JSON.stringify({ id: "42", created: true }, null, 2),
    },
  },
];

const CATEGORIES = ["All", ...Array.from(new Set(API_ENDPOINTS.map((e) => e.category)))];

const CATEGORY_ICONS: Record<string, React.ReactNode> = {
  Analytics: <BarChart3 className="w-4 h-4" />,
  Network: <Globe className="w-4 h-4" />,
  Payments: <Zap className="w-4 h-4" />,
  Metrics: <Activity className="w-4 h-4" />,
  Alerts: <Shield className="w-4 h-4" />,
};

// ---------------------------------------------------------------------------
// Small reusable components
// ---------------------------------------------------------------------------

function MethodBadge({ method }: { method: string }) {
  return (
    <span
      className={`inline-block text-xs font-mono font-bold px-2 py-0.5 rounded ${METHOD_COLORS[method] ?? "bg-slate-700 text-slate-300"}`}
    >
      {method}
    </span>
  );
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // clipboard API not available in some environments
    }
  };

  return (
    <button
      type="button"
      aria-label={copied ? "Copied" : "Copy to clipboard"}
      onClick={handleCopy}
      className="p-1.5 rounded hover:bg-slate-600 text-slate-400 hover:text-slate-200 transition-colors"
    >
      {copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
    </button>
  );
}

function CodeBlock({ code, label }: { code: string; label: string }) {
  return (
    <div className="rounded-lg overflow-hidden border border-slate-700">
      <div className="flex items-center justify-between px-3 py-1.5 bg-slate-800 border-b border-slate-700">
        <span className="text-xs font-medium text-slate-400">{label}</span>
        <CopyButton text={code} />
      </div>
      <pre className="p-4 bg-slate-900 text-sm text-slate-300 overflow-x-auto leading-relaxed">
        <code>{code}</code>
      </pre>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Endpoint card (collapsible)
// ---------------------------------------------------------------------------

function EndpointCard({ endpoint }: { endpoint: ApiEndpoint }) {
  const [open, setOpen] = useState(false);

  return (
    <div className="rounded-xl border border-slate-700 bg-slate-800/50 overflow-hidden">
      {/* Header row */}
      <button
        type="button"
        className="w-full flex items-center gap-3 px-4 py-3.5 text-left hover:bg-slate-700/40 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500"
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
      >
        <MethodBadge method={endpoint.method} />
        <span className="flex-1 font-mono text-sm text-slate-200 min-w-0 truncate">{endpoint.path}</span>
        {endpoint.auth && (
          <span
            title="Requires authentication"
            className="hidden sm:flex items-center gap-1 text-xs text-amber-400 bg-amber-500/10 border border-amber-500/30 px-2 py-0.5 rounded"
          >
            <Shield className="w-3 h-3" />
            Auth
          </span>
        )}
        <span className="hidden sm:block text-sm text-slate-400 ml-2">{endpoint.summary}</span>
        {open ? (
          <ChevronDown className="w-4 h-4 text-slate-400 ml-auto shrink-0" />
        ) : (
          <ChevronRight className="w-4 h-4 text-slate-400 ml-auto shrink-0" />
        )}
      </button>

      {/* Expanded detail */}
      {open && (
        <div className="px-4 pb-5 pt-2 border-t border-slate-700 space-y-5">
          <p className="text-sm text-slate-300">{endpoint.description}</p>

          {/* Parameters */}
          {endpoint.params && endpoint.params.length > 0 && (
            <div>
              <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-500 mb-2">Parameters</h3>
              <div className="overflow-x-auto">
                <table className="w-full text-sm border-collapse">
                  <thead>
                    <tr className="text-left text-xs text-slate-500 border-b border-slate-700">
                      <th className="pb-1.5 pr-4 font-medium">Name</th>
                      <th className="pb-1.5 pr-4 font-medium">Type</th>
                      <th className="pb-1.5 pr-4 font-medium">Required</th>
                      <th className="pb-1.5 font-medium">Description</th>
                    </tr>
                  </thead>
                  <tbody>
                    {endpoint.params.map((p) => (
                      <tr key={p.name} className="border-b border-slate-700/50 last:border-0">
                        <td className="py-1.5 pr-4 font-mono text-indigo-300">{p.name}</td>
                        <td className="py-1.5 pr-4 text-slate-400">{p.type}</td>
                        <td className="py-1.5 pr-4">
                          {p.required ? (
                            <span className="text-red-400 text-xs">required</span>
                          ) : (
                            <span className="text-slate-500 text-xs">optional</span>
                          )}
                        </td>
                        <td className="py-1.5 text-slate-400 text-xs">{p.description}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* Examples */}
          <div className="space-y-3">
            {endpoint.example.request && (
              <CodeBlock code={endpoint.example.request} label="Request body" />
            )}
            <CodeBlock code={endpoint.example.response} label="Example response" />
          </div>

          {/* Try it link */}
          <div>
            <Link
              href={`/api-docs/playground?endpoint=${encodeURIComponent(endpoint.path)}&method=${endpoint.method}`}
              className="inline-flex items-center gap-2 text-sm text-indigo-400 hover:text-indigo-300 transition-colors"
            >
              <FlaskConical className="w-4 h-4" />
              Try in Playground
              <ExternalLink className="w-3 h-3" />
            </Link>
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function APIDocumentationPortal() {
  const [search, setSearch] = useState("");
  const [activeCategory, setActiveCategory] = useState("All");

  const filtered = useMemo(() => {
    const q = search.toLowerCase().trim();
    return API_ENDPOINTS.filter((ep) => {
      const matchesCategory = activeCategory === "All" || ep.category === activeCategory;
      const matchesSearch =
        !q ||
        ep.path.toLowerCase().includes(q) ||
        ep.summary.toLowerCase().includes(q) ||
        ep.description.toLowerCase().includes(q) ||
        ep.method.toLowerCase().includes(q);
      return matchesCategory && matchesSearch;
    });
  }, [search, activeCategory]);

  return (
    <div className="min-h-screen bg-slate-900 text-slate-100">
      {/* ── Hero header ─────────────────────────────────────────────────────── */}
      <header className="border-b border-slate-700 bg-slate-900/80 backdrop-blur-sm sticky top-0 z-10">
        <div className="max-w-6xl mx-auto px-4 py-4 flex items-center justify-between gap-4 flex-wrap">
          <div className="flex items-center gap-2.5">
            <BookOpen className="w-6 h-6 text-indigo-400" aria-hidden="true" />
            <h1 className="text-lg font-bold text-slate-100">API Documentation</h1>
            <span className="text-xs font-mono bg-indigo-500/20 text-indigo-300 border border-indigo-500/30 px-2 py-0.5 rounded">
              v1
            </span>
          </div>
          <nav className="flex items-center gap-3" aria-label="API docs navigation">
            <Link
              href="/api-docs/examples"
              className="flex items-center gap-1.5 text-sm text-slate-300 hover:text-white transition-colors"
            >
              <Code2 className="w-4 h-4" />
              Code Examples
            </Link>
            <Link
              href="/api-docs/playground"
              className="flex items-center gap-1.5 text-sm bg-indigo-600 hover:bg-indigo-500 text-white px-3 py-1.5 rounded-lg transition-colors"
            >
              <FlaskConical className="w-4 h-4" />
              Playground
            </Link>
          </nav>
        </div>
      </header>

      <main className="max-w-6xl mx-auto px-4 py-8 space-y-8">
        {/* ── Intro card ────────────────────────────────────────────────────── */}
        <section
          className="rounded-2xl bg-gradient-to-br from-indigo-900/40 to-slate-800/60 border border-indigo-700/30 p-6"
          aria-labelledby="intro-heading"
        >
          <h2 id="intro-heading" className="text-2xl font-bold mb-2">
            PayRaider REST API
          </h2>
          <p className="text-slate-300 mb-4 max-w-2xl">
            Build on top of real-time Stellar payment intelligence. The API provides corridor analytics, payment routing,
            network health metrics, and on-chain snapshot verification.
          </p>
          <div className="flex flex-wrap gap-4 text-sm">
            <div className="flex items-center gap-2 text-slate-300">
              <span className="w-2 h-2 rounded-full bg-emerald-400" aria-hidden="true" />
              Base URL:&nbsp;
              <code className="font-mono text-indigo-300">https://api.payraider.io</code>
            </div>
            <div className="flex items-center gap-2 text-slate-300">
              <span className="w-2 h-2 rounded-full bg-amber-400" aria-hidden="true" />
              Auth:&nbsp;
              <code className="font-mono text-indigo-300">Bearer &lt;token&gt;</code>
            </div>
          </div>
        </section>

        {/* ── Quick links ───────────────────────────────────────────────────── */}
        <section aria-label="Quick links" className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          {[
            {
              icon: <FlaskConical className="w-5 h-5 text-indigo-400" />,
              title: "API Playground",
              desc: "Test endpoints live in your browser with real responses.",
              href: "/api-docs/playground",
            },
            {
              icon: <Code2 className="w-5 h-5 text-emerald-400" />,
              title: "Code Examples",
              desc: "Copy-paste snippets in TypeScript, Python, and cURL.",
              href: "/api-docs/examples",
            },
            {
              icon: <Activity className="w-5 h-5 text-amber-400" />,
              title: "API Status",
              desc: "Check real-time availability and incident history.",
              href: "/health",
            },
          ].map(({ icon, title, desc, href }) => (
            <Link
              key={title}
              href={href}
              className="group flex flex-col gap-2 rounded-xl border border-slate-700 bg-slate-800/60 p-4 hover:border-slate-500 hover:bg-slate-800 transition-all"
            >
              {icon}
              <span className="font-semibold text-slate-100 group-hover:text-white">{title}</span>
              <span className="text-sm text-slate-400">{desc}</span>
            </Link>
          ))}
        </section>

        {/* ── Endpoint browser ──────────────────────────────────────────────── */}
        <section aria-labelledby="endpoints-heading">
          <div className="flex flex-col sm:flex-row sm:items-center gap-4 mb-5">
            <h2 id="endpoints-heading" className="text-xl font-bold">
              Endpoints
            </h2>
            {/* Search */}
            <div className="relative flex-1 max-w-md">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none" aria-hidden="true" />
              <input
                type="search"
                placeholder="Search endpoints…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                aria-label="Search endpoints"
                className="w-full bg-slate-800 border border-slate-700 text-slate-200 placeholder-slate-500 rounded-lg pl-9 pr-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
              />
            </div>
          </div>

          {/* Category filter tabs */}
          <div
            className="flex flex-wrap gap-2 mb-5"
            role="tablist"
            aria-label="Filter endpoints by category"
          >
            {CATEGORIES.map((cat) => (
              <button
                key={cat}
                type="button"
                role="tab"
                aria-selected={activeCategory === cat}
                onClick={() => setActiveCategory(cat)}
                className={`flex items-center gap-1.5 text-sm px-3 py-1.5 rounded-lg border transition-colors ${
                  activeCategory === cat
                    ? "bg-indigo-600 border-indigo-600 text-white"
                    : "bg-slate-800 border-slate-700 text-slate-400 hover:border-slate-500 hover:text-slate-200"
                }`}
              >
                {cat !== "All" && CATEGORY_ICONS[cat]}
                {cat}
              </button>
            ))}
          </div>

          {/* Endpoint list */}
          {filtered.length === 0 ? (
            <div className="text-center py-16 text-slate-500">
              <Search className="w-10 h-10 mx-auto mb-3 opacity-40" aria-hidden="true" />
              <p>No endpoints match your search.</p>
            </div>
          ) : (
            <div className="space-y-3" role="list" aria-label="API endpoints">
              {filtered.map((ep) => (
                <div key={`${ep.method}-${ep.path}`} role="listitem">
                  <EndpointCard endpoint={ep} />
                </div>
              ))}
            </div>
          )}
        </section>

        {/* ── Authentication section ────────────────────────────────────────── */}
        <section
          aria-labelledby="auth-heading"
          className="rounded-xl border border-slate-700 bg-slate-800/50 p-5 space-y-3"
        >
          <h2 id="auth-heading" className="text-lg font-bold flex items-center gap-2">
            <Shield className="w-5 h-5 text-amber-400" aria-hidden="true" />
            Authentication
          </h2>
          <p className="text-sm text-slate-300">
            Protected endpoints require a Bearer token in the <code className="font-mono text-indigo-300">Authorization</code>{" "}
            header. Obtain tokens from your account settings.
          </p>
          <CodeBlock
            code={`curl -H "Authorization: Bearer <your_token>" \\
  https://api.payraider.io/api/v1/payments/history?account=G...`}
            label="cURL example"
          />
        </section>

        {/* ── Rate limits ───────────────────────────────────────────────────── */}
        <section
          aria-labelledby="rate-limits-heading"
          className="rounded-xl border border-slate-700 bg-slate-800/50 p-5"
        >
          <h2 id="rate-limits-heading" className="text-lg font-bold mb-3 flex items-center gap-2">
            <Zap className="w-5 h-5 text-indigo-400" aria-hidden="true" />
            Rate Limits
          </h2>
          <div className="overflow-x-auto">
            <table className="w-full text-sm text-left">
              <thead>
                <tr className="border-b border-slate-700 text-xs text-slate-500">
                  <th className="pb-2 pr-6 font-medium">Plan</th>
                  <th className="pb-2 pr-6 font-medium">Requests / min</th>
                  <th className="pb-2 font-medium">Burst</th>
                </tr>
              </thead>
              <tbody className="text-slate-300">
                {[
                  { plan: "Free", rpm: "60", burst: "10" },
                  { plan: "Pro", rpm: "600", burst: "100" },
                  { plan: "Enterprise", rpm: "Unlimited", burst: "Unlimited" },
                ].map((row) => (
                  <tr key={row.plan} className="border-b border-slate-700/50 last:border-0">
                    <td className="py-2 pr-6 font-medium">{row.plan}</td>
                    <td className="py-2 pr-6">{row.rpm}</td>
                    <td className="py-2">{row.burst}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      </main>
    </div>
  );
}
