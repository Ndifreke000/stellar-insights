"use client";

import React, { useCallback, useEffect, useState } from "react";
import { Gauge, RefreshCw } from "lucide-react";
import { FRONTEND_METRICS_ENDPOINT } from "@/lib/monitoring";

interface MetricSummary {
  name: string;
  count: number;
  p50: number;
  p75: number;
  p95: number;
  budget: number | null;
  status: "good" | "poor" | "n/a";
}

interface PageSummary {
  path: string;
  metric: string;
  p75: number;
  count: number;
}

interface FrontendSummary {
  metrics: MetricSummary[];
  slowest_pages: PageSummary[];
  error_count: number;
}

const LABELS: Record<string, string> = {
  "web-vitals-lcp": "LCP",
  "web-vitals-fid": "FID",
  "web-vitals-inp": "INP",
  "web-vitals-cls": "CLS",
  "web-vitals-fcp": "FCP",
  "web-vitals-ttfb": "TTFB",
  "page-load-time": "Page load",
  "api-response-time": "API latency",
  "api-latency": "API latency (proxy)",
};

const fmt = (name: string, v: number) =>
  name === "web-vitals-cls" ? v.toFixed(3) : `${Math.round(v)} ms`;

/**
 * Real user monitoring summary served by the backend (`GET /api/metrics/frontend`):
 * p50/p75/p95 per metric against performance budgets, plus the slowest pages.
 */
export function LivePerformancePanel() {
  const [data, setData] = useState<FrontendSummary | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const res = await fetch(FRONTEND_METRICS_ENDPOINT);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setData(await res.json());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    load();
    const id = setInterval(load, 30000);
    return () => clearInterval(id);
  }, [load]);

  const known = data?.metrics.filter((m) => LABELS[m.name]) ?? [];

  return (
    <section className="bg-white dark:bg-slate-800 p-6 rounded-xl border border-gray-200 dark:border-slate-700 shadow-sm mb-8">
      <div className="flex items-center justify-between mb-6">
        <h3 className="text-lg font-semibold dark:text-white flex items-center gap-2">
          <Gauge size={18} className="text-blue-500" />
          Real User Performance (p75 vs budget)
        </h3>
        <button
          onClick={load}
          className="text-muted-foreground hover:text-foreground"
          aria-label="Refresh"
        >
          <RefreshCw size={16} />
        </button>
      </div>

      {error && (
        <p className="text-sm text-red-500">
          Could not load live metrics: {error}
        </p>
      )}
      {data && known.length === 0 && (
        <p className="text-sm text-muted-foreground">
          No real-user metrics collected yet.
        </p>
      )}

      {known.length > 0 && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
          {known.map((m) => (
            <div
              key={m.name}
              className="p-4 rounded-lg border border-gray-200 dark:border-slate-700"
            >
              <div className="flex items-center justify-between">
                <span className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
                  {LABELS[m.name]}
                </span>
                <span
                  className={`text-xs font-semibold ${
                    m.status === "good"
                      ? "text-green-500"
                      : m.status === "poor"
                        ? "text-red-500"
                        : "text-muted-foreground"
                  }`}
                >
                  {m.status}
                </span>
              </div>
              <p className="text-2xl font-bold dark:text-white mt-1">
                {fmt(m.name, m.p75)}
              </p>
              <p className="text-xs text-muted-foreground mt-1">
                p50 {fmt(m.name, m.p50)} · p95 {fmt(m.name, m.p95)}
                {m.budget !== null && ` · budget ${fmt(m.name, m.budget)}`}
              </p>
              <p className="text-xs text-muted-foreground">{m.count} samples</p>
            </div>
          ))}
        </div>
      )}

      {data && data.slowest_pages.length > 0 && (
        <div className="overflow-x-auto">
          <h4 className="text-sm font-semibold mb-2 dark:text-white">
            Slowest pages
          </h4>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-muted-foreground">
                <th className="py-1">Path</th>
                <th className="py-1">Metric</th>
                <th className="py-1 text-right">p75</th>
                <th className="py-1 text-right">Samples</th>
              </tr>
            </thead>
            <tbody>
              {data.slowest_pages.map((p) => (
                <tr
                  key={`${p.metric}-${p.path}`}
                  className="border-t border-gray-100 dark:border-slate-700"
                >
                  <td className="py-1 font-mono dark:text-white">{p.path}</td>
                  <td className="py-1">{LABELS[p.metric] ?? p.metric}</td>
                  <td className="py-1 text-right">{fmt(p.metric, p.p75)}</td>
                  <td className="py-1 text-right">{p.count}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {data && (
        <p className="text-xs text-muted-foreground mt-4">
          Client errors reported: {data.error_count}
        </p>
      )}
    </section>
  );
}
