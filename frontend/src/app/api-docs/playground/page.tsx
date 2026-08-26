"use client";

/**
 * Issue #2131 — API Documentation Portal: Playground sub-page
 *
 * An interactive request builder that lets users fire real or mocked API calls
 * and inspect the response without leaving the browser.
 */

import React, { useState, useCallback, useEffect, Suspense } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import {
  Play,
  BookOpen,
  Copy,
  Check,
  ChevronDown,
  Loader2,
  ArrowLeft,
  Code2,
} from "lucide-react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface RequestConfig {
  method: string;
  endpoint: string;
  headers: string;
  body: string;
}

interface ResponseState {
  status: number | null;
  statusText: string;
  body: string;
  duration: number | null;
  error: string | null;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const HTTP_METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE"];

const PRESET_REQUESTS: Array<{ label: string; config: RequestConfig }> = [
  {
    label: "List corridors",
    config: {
      method: "GET",
      endpoint: "/api/v1/analytics/corridors?limit=5",
      headers: '{\n  "Accept": "application/json"\n}',
      body: "",
    },
  },
  {
    label: "Network health",
    config: {
      method: "GET",
      endpoint: "/api/v1/network/health",
      headers: '{\n  "Accept": "application/json"\n}',
      body: "",
    },
  },
  {
    label: "Dashboard metrics",
    config: {
      method: "GET",
      endpoint: "/api/v1/metrics/dashboard",
      headers: '{\n  "Accept": "application/json"\n}',
      body: "",
    },
  },
  {
    label: "Route payment",
    config: {
      method: "POST",
      endpoint: "/api/v1/payments/route",
      headers: '{\n  "Content-Type": "application/json",\n  "Authorization": "Bearer <token>"\n}',
      body: JSON.stringify({ source_asset: "USD", dest_asset: "PHP", amount: 100 }, null, 2),
    },
  },
  {
    label: "Snapshot epoch 1",
    config: {
      method: "GET",
      endpoint: "/api/v1/analytics/snapshots/1",
      headers: '{\n  "Accept": "application/json"\n}',
      body: "",
    },
  },
];

/**
 * Mock response generator — used when running in the browser (no real backend
 * available in the demo environment). Returns plausible data for each route.
 */
function getMockResponse(method: string, endpoint: string): { status: number; body: object } {
  const path = endpoint.split("?")[0];

  if (path.includes("/analytics/corridors")) {
    return {
      status: 200,
      body: {
        corridors: [
          { id: "USD-PHP", reliability: 0.98, avg_latency_ms: 4200, volume_24h: 1250000 },
          { id: "EUR-MXN", reliability: 0.95, avg_latency_ms: 5100, volume_24h: 840000 },
          { id: "USD-MXN", reliability: 0.97, avg_latency_ms: 4600, volume_24h: 620000 },
        ],
        total: 3,
        offset: 0,
      },
    };
  }

  if (path.match(/\/analytics\/snapshots\/\d+/)) {
    const epoch = parseInt(path.split("/").pop() ?? "1", 10);
    return {
      status: 200,
      body: { epoch, hash: "a3f2c1d4e5b6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2", timestamp: 1722000000, verified: true },
    };
  }

  if (path.includes("/network/health")) {
    return {
      status: 200,
      body: { status: "healthy", ledger_sequence: 51234567, close_time_ms: 5100, quorum_health: 1.0 },
    };
  }

  if (path.includes("/network/graph")) {
    return {
      status: 200,
      body: {
        nodes: [{ id: "GXYZ", type: "anchor", label: "StellarX" }],
        edges: [{ source: "GXYZ", target: "GABC", weight: 0.85 }],
      },
    };
  }

  if (path.includes("/payments/route") && method === "POST") {
    return {
      status: 200,
      body: { route: ["USD", "XLM", "PHP"], estimated_fee_pct: 0.11, success_probability: 0.97, estimated_time_ms: 4800 },
    };
  }

  if (path.includes("/payments/history")) {
    return {
      status: 200,
      body: {
        payments: [{ id: "pay_abc", amount: "100", asset: "USD", status: "success", latency_ms: 4200 }],
        next_cursor: "cur_xyz",
      },
    };
  }

  if (path.includes("/metrics/dashboard")) {
    return {
      status: 200,
      body: { total_payments_24h: 142000, success_rate_24h: 0.972, avg_fee_pct: 0.13, active_corridors: 38 },
    };
  }

  if (path.includes("/metrics/latency")) {
    return {
      status: 200,
      body: { p50_ms: 4100, p95_ms: 8300, p99_ms: 14200, corridor: "all", range: "24h" },
    };
  }

  if (path.includes("/alerts") && method === "POST") {
    return { status: 201, body: { id: "42", created: true } };
  }

  if (path.includes("/alerts")) {
    return {
      status: 200,
      body: {
        alerts: [{ id: "1", name: "High latency", condition: "latency_ms > 10000", active: true }],
      },
    };
  }

  return { status: 404, body: { error: "Not found", path } };
}

// ---------------------------------------------------------------------------
// Copy button
// ---------------------------------------------------------------------------

function CopyButton({ text, className = "" }: { text: string; className?: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // clipboard not available
    }
  };

  return (
    <button
      type="button"
      aria-label={copied ? "Copied" : "Copy to clipboard"}
      onClick={handleCopy}
      className={`p-1.5 rounded hover:bg-slate-600 text-slate-400 hover:text-slate-200 transition-colors ${className}`}
    >
      {copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
    </button>
  );
}

// ---------------------------------------------------------------------------
// Status badge
// ---------------------------------------------------------------------------

function StatusBadge({ status }: { status: number }) {
  const color =
    status >= 500
      ? "bg-red-500/10 text-red-400 border-red-500/30"
      : status >= 400
      ? "bg-amber-500/10 text-amber-400 border-amber-500/30"
      : status >= 300
      ? "bg-blue-500/10 text-blue-400 border-blue-500/30"
      : "bg-emerald-500/10 text-emerald-400 border-emerald-500/30";

  return (
    <span className={`inline-block text-xs font-mono font-bold px-2 py-0.5 rounded border ${color}`}>
      {status}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Playground inner component (needs useSearchParams, so wrapped in Suspense)
// ---------------------------------------------------------------------------

function PlaygroundInner() {
  const searchParams = useSearchParams();

  const [config, setConfig] = useState<RequestConfig>({
    method: searchParams.get("method") ?? "GET",
    endpoint: searchParams.get("endpoint") ?? "/api/v1/analytics/corridors",
    headers: '{\n  "Accept": "application/json"\n}',
    body: "",
  });

  const [response, setResponse] = useState<ResponseState>({
    status: null,
    statusText: "",
    body: "",
    duration: null,
    error: null,
  });

  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<"response" | "headers">("response");
  const [presetsOpen, setPresetsOpen] = useState(false);

  // Sync endpoint/method from query params when navigating from the docs
  useEffect(() => {
    const ep = searchParams.get("endpoint");
    const m = searchParams.get("method");
    if (ep) setConfig((c) => ({ ...c, endpoint: ep, method: m ?? c.method }));
  }, [searchParams]);

  const applyPreset = useCallback((cfg: RequestConfig) => {
    setConfig(cfg);
    setPresetsOpen(false);
    setResponse({ status: null, statusText: "", body: "", duration: null, error: null });
  }, []);

  const sendRequest = useCallback(async () => {
    setLoading(true);
    setResponse({ status: null, statusText: "", body: "", duration: null, error: null });

    const start = performance.now();
    // Simulate network latency then return mock data
    await new Promise((r) => setTimeout(r, 300 + Math.random() * 400));

    try {
      const { status, body } = getMockResponse(config.method, config.endpoint);
      const duration = Math.round(performance.now() - start);
      setResponse({
        status,
        statusText: status < 400 ? "OK" : status < 500 ? "Bad Request" : "Internal Server Error",
        body: JSON.stringify(body, null, 2),
        duration,
        error: null,
      });
    } catch (err) {
      setResponse({
        status: null,
        statusText: "",
        body: "",
        duration: null,
        error: err instanceof Error ? err.message : "Unknown error",
      });
    } finally {
      setLoading(false);
    }
  }, [config]);

  return (
    <div className="min-h-screen bg-slate-900 text-slate-100">
      {/* Header */}
      <header className="border-b border-slate-700 bg-slate-900/80 backdrop-blur-sm sticky top-0 z-10">
        <div className="max-w-7xl mx-auto px-4 py-3 flex items-center gap-4 flex-wrap">
          <Link
            href="/api-docs"
            className="flex items-center gap-1.5 text-sm text-slate-400 hover:text-slate-200 transition-colors"
          >
            <ArrowLeft className="w-4 h-4" />
            Back to Docs
          </Link>
          <div className="flex items-center gap-2">
            <BookOpen className="w-5 h-5 text-indigo-400" aria-hidden="true" />
            <h1 className="text-base font-bold">API Playground</h1>
          </div>
          <Link
            href="/api-docs/examples"
            className="ml-auto flex items-center gap-1.5 text-sm text-slate-300 hover:text-white transition-colors"
          >
            <Code2 className="w-4 h-4" />
            Code Examples
          </Link>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-4 py-6">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* ── Left panel: Request Builder ────────────────────────────────── */}
          <section aria-labelledby="request-heading" className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 id="request-heading" className="text-lg font-bold">
                Request
              </h2>
              {/* Presets dropdown */}
              <div className="relative">
                <button
                  type="button"
                  onClick={() => setPresetsOpen((o) => !o)}
                  aria-expanded={presetsOpen}
                  aria-haspopup="listbox"
                  className="flex items-center gap-1.5 text-sm text-slate-300 hover:text-white bg-slate-800 border border-slate-700 px-3 py-1.5 rounded-lg transition-colors"
                >
                  Presets
                  <ChevronDown className="w-3.5 h-3.5" />
                </button>
                {presetsOpen && (
                  <ul
                    role="listbox"
                    aria-label="Request presets"
                    className="absolute right-0 top-full mt-1 z-20 bg-slate-800 border border-slate-700 rounded-xl shadow-xl w-52 overflow-hidden"
                  >
                    {PRESET_REQUESTS.map((p) => (
                      <li key={p.label} role="option" aria-selected={false}>
                        <button
                          type="button"
                          className="w-full text-left text-sm px-3 py-2 hover:bg-slate-700 text-slate-300 hover:text-white transition-colors"
                          onClick={() => applyPreset(p.config)}
                        >
                          {p.label}
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </div>

            {/* Method + URL */}
            <div className="flex gap-2">
              <select
                value={config.method}
                onChange={(e) => setConfig((c) => ({ ...c, method: e.target.value }))}
                aria-label="HTTP method"
                className="bg-slate-800 border border-slate-700 text-slate-200 text-sm rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-indigo-500 cursor-pointer"
              >
                {HTTP_METHODS.map((m) => (
                  <option key={m} value={m}>
                    {m}
                  </option>
                ))}
              </select>
              <input
                type="text"
                value={config.endpoint}
                onChange={(e) => setConfig((c) => ({ ...c, endpoint: e.target.value }))}
                aria-label="Endpoint path"
                placeholder="/api/v1/..."
                className="flex-1 bg-slate-800 border border-slate-700 text-slate-200 text-sm font-mono rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-indigo-500"
              />
            </div>

            {/* Headers */}
            <div>
              <label htmlFor="headers-input" className="block text-xs font-medium text-slate-400 mb-1.5">
                Headers (JSON)
              </label>
              <textarea
                id="headers-input"
                value={config.headers}
                onChange={(e) => setConfig((c) => ({ ...c, headers: e.target.value }))}
                rows={4}
                spellCheck={false}
                className="w-full bg-slate-800 border border-slate-700 text-slate-200 text-xs font-mono rounded-lg px-3 py-2 resize-y focus:outline-none focus:ring-2 focus:ring-indigo-500"
              />
            </div>

            {/* Body (only shown for non-GET) */}
            {config.method !== "GET" && config.method !== "DELETE" && (
              <div>
                <label htmlFor="body-input" className="block text-xs font-medium text-slate-400 mb-1.5">
                  Body (JSON)
                </label>
                <textarea
                  id="body-input"
                  value={config.body}
                  onChange={(e) => setConfig((c) => ({ ...c, body: e.target.value }))}
                  rows={6}
                  spellCheck={false}
                  placeholder='{\n  "key": "value"\n}'
                  className="w-full bg-slate-800 border border-slate-700 text-slate-200 text-xs font-mono rounded-lg px-3 py-2 resize-y focus:outline-none focus:ring-2 focus:ring-indigo-500"
                />
              </div>
            )}

            {/* Send button */}
            <button
              type="button"
              onClick={sendRequest}
              disabled={loading}
              className="w-full flex items-center justify-center gap-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-60 disabled:cursor-not-allowed text-white font-semibold py-2.5 rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-indigo-500"
            >
              {loading ? (
                <Loader2 className="w-4 h-4 animate-spin" aria-hidden="true" />
              ) : (
                <Play className="w-4 h-4" aria-hidden="true" />
              )}
              {loading ? "Sending…" : "Send Request"}
            </button>

            <p className="text-xs text-slate-500 text-center">
              Requests are resolved against the mock API. Real calls require a valid token.
            </p>
          </section>

          {/* ── Right panel: Response ──────────────────────────────────────── */}
          <section aria-labelledby="response-heading" className="space-y-4">
            <h2 id="response-heading" className="text-lg font-bold">
              Response
            </h2>

            {!response.status && !response.error && !loading && (
              <div className="flex flex-col items-center justify-center h-64 rounded-xl border border-slate-700 bg-slate-800/40 text-slate-500">
                <Play className="w-10 h-10 mb-3 opacity-30" aria-hidden="true" />
                <p className="text-sm">Hit &ldquo;Send Request&rdquo; to see the response here.</p>
              </div>
            )}

            {loading && (
              <div className="flex flex-col items-center justify-center h-64 rounded-xl border border-slate-700 bg-slate-800/40 text-slate-400">
                <Loader2 className="w-8 h-8 mb-3 animate-spin text-indigo-400" aria-hidden="true" />
                <p className="text-sm">Waiting for response…</p>
              </div>
            )}

            {response.error && (
              <div className="rounded-xl border border-red-700/50 bg-red-900/20 p-4 text-sm text-red-300">
                <strong>Error:</strong> {response.error}
              </div>
            )}

            {response.status !== null && !loading && (
              <div className="rounded-xl border border-slate-700 bg-slate-800/50 overflow-hidden">
                {/* Status bar */}
                <div className="flex items-center gap-3 px-4 py-2.5 bg-slate-800 border-b border-slate-700">
                  <StatusBadge status={response.status} />
                  <span className="text-xs text-slate-400">{response.statusText}</span>
                  {response.duration !== null && (
                    <span className="ml-auto text-xs text-slate-500">{response.duration} ms</span>
                  )}
                  <CopyButton text={response.body} />
                </div>

                {/* Tabs */}
                <div className="flex border-b border-slate-700">
                  {(["response", "headers"] as const).map((tab) => (
                    <button
                      key={tab}
                      type="button"
                      role="tab"
                      aria-selected={activeTab === tab}
                      onClick={() => setActiveTab(tab)}
                      className={`px-4 py-2 text-xs font-medium capitalize transition-colors ${
                        activeTab === tab
                          ? "border-b-2 border-indigo-500 text-indigo-300"
                          : "text-slate-400 hover:text-slate-200"
                      }`}
                    >
                      {tab}
                    </button>
                  ))}
                </div>

                {/* Tab content */}
                {activeTab === "response" && (
                  <pre className="p-4 text-xs font-mono text-slate-300 overflow-x-auto max-h-96 leading-relaxed">
                    <code>{response.body}</code>
                  </pre>
                )}

                {activeTab === "headers" && (
                  <pre className="p-4 text-xs font-mono text-slate-300 overflow-x-auto max-h-96 leading-relaxed">
                    <code>{`Content-Type: application/json\nX-Request-Id: req_${Math.random().toString(36).slice(2, 10)}\nX-RateLimit-Remaining: 599`}</code>
                  </pre>
                )}
              </div>
            )}
          </section>
        </div>
      </main>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page export wrapped in Suspense (required for useSearchParams in Next.js)
// ---------------------------------------------------------------------------

export default function APIPlayground() {
  return (
    <Suspense
      fallback={
        <div className="min-h-screen bg-slate-900 flex items-center justify-center text-slate-400">
          <Loader2 className="w-8 h-8 animate-spin" aria-hidden="true" />
        </div>
      }
    >
      <PlaygroundInner />
    </Suspense>
  );
}
