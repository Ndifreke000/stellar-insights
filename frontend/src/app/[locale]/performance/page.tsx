"use client";

import React, { useEffect, useState } from "react";
import {
  BarChart,
  Bar,
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from "recharts";
import type { Metric, AppError } from "@/lib/monitoring";

interface WebVitalSummary {
  name: string;
  value: number;
  rating: "good" | "needs-improvement" | "poor";
  threshold: { good: number; poor: number };
  unit: string;
}

interface ApiLatencyEntry {
  endpoint: string;
  latency: number;
}

// Web Vitals thresholds (ms or score)
const VITALS_CONFIG: Record<string, { good: number; poor: number; unit: string }> = {
  lcp: { good: 2500, poor: 4000, unit: "ms" },
  fid: { good: 100, poor: 300, unit: "ms" },
  inp: { good: 200, poor: 500, unit: "ms" },
  cls: { good: 0.1, poor: 0.25, unit: "" },
  fcp: { good: 1800, poor: 3000, unit: "ms" },
  ttfb: { good: 800, poor: 1800, unit: "ms" },
};

function getRating(name: string, value: number): "good" | "needs-improvement" | "poor" {
  const cfg = VITALS_CONFIG[name];
  if (!cfg) return "good";
  if (value <= cfg.good) return "good";
  if (value <= cfg.poor) return "needs-improvement";
  return "poor";
}

const RATING_COLOR: Record<string, string> = {
  good: "text-green-400",
  "needs-improvement": "text-yellow-400",
  poor: "text-red-400",
};

const RATING_BG: Record<string, string> = {
  good: "bg-green-500/10 border-green-500/20",
  "needs-improvement": "bg-yellow-500/10 border-yellow-500/20",
  poor: "bg-red-500/10 border-red-500/20",
};

export default function PerformancePage() {
  const [vitals, setVitals] = useState<WebVitalSummary[]>([]);
  const [apiLatencies, setApiLatencies] = useState<ApiLatencyEntry[]>([]);
  const [errorCount, setErrorCount] = useState(0);
  const [errorTimeline, setErrorTimeline] = useState<{ time: string; count: number }[]>([]);

  useEffect(() => {
    const rawMetrics: Metric[] = JSON.parse(localStorage.getItem("mon_metrics") || "[]");
    const rawErrors: AppError[] = JSON.parse(localStorage.getItem("mon_errors") || "[]");

    // Aggregate Web Vitals (latest value per metric)
    const vitalsMap = new Map<string, number>();
    for (const m of rawMetrics) {
      if (m.name.startsWith("web-vitals-")) {
        const key = m.name.replace("web-vitals-", "");
        vitalsMap.set(key, m.value);
      }
    }
    const vitalsSummary: WebVitalSummary[] = Array.from(vitalsMap.entries()).map(([name, value]) => ({
      name: name.toUpperCase(),
      value,
      rating: getRating(name, value),
      threshold: VITALS_CONFIG[name] ?? { good: 0, poor: 0 },
      unit: VITALS_CONFIG[name]?.unit ?? "",
    }));
    setVitals(vitalsSummary);

    // Aggregate API latencies (average per endpoint)
    const latencyMap = new Map<string, number[]>();
    for (const m of rawMetrics) {
      if (m.name === "api-latency" && m.metadata?.endpoint) {
        const ep = m.metadata.endpoint as string;
        if (!latencyMap.has(ep)) latencyMap.set(ep, []);
        latencyMap.get(ep)!.push(m.value);
      }
    }
    const latencies: ApiLatencyEntry[] = Array.from(latencyMap.entries()).map(([endpoint, values]) => ({
      endpoint,
      latency: Math.round(values.reduce((a, b) => a + b, 0) / values.length),
    }));
    setApiLatencies(latencies);

    // Error count and timeline (last 12 hours, bucketed by hour)
    setErrorCount(rawErrors.length);
    const buckets = new Map<string, number>();
    for (const e of rawErrors) {
      const hour = new Date(e.timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      buckets.set(hour, (buckets.get(hour) ?? 0) + 1);
    }
    setErrorTimeline(Array.from(buckets.entries()).map(([time, count]) => ({ time, count })));
  }, []);

  return (
    <div className="max-w-5xl mx-auto space-y-8">
      <div>
        <h1 className="text-3xl font-bold">Performance</h1>
        <p className="text-muted-foreground mt-1 text-sm">Real User Monitoring — Web Vitals, API latency, and error rates</p>
      </div>

      {/* Web Vitals */}
      <section>
        <h2 className="text-lg font-semibold mb-4">Core Web Vitals</h2>
        {vitals.length === 0 ? (
          <p className="text-muted-foreground text-sm">No Web Vitals recorded yet. Navigate around the app to collect data.</p>
        ) : (
          <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
            {vitals.map((v) => (
              <div key={v.name} className={`glass-card rounded-2xl p-5 border ${RATING_BG[v.rating]}`}>
                <div className="text-xs font-mono text-muted-foreground uppercase tracking-widest mb-1">{v.name}</div>
                <div className={`text-2xl font-bold tabular-nums ${RATING_COLOR[v.rating]}`}>
                  {v.unit === "ms" ? `${Math.round(v.value)}ms` : v.value.toFixed(3)}
                </div>
                <div className={`text-xs mt-1 capitalize ${RATING_COLOR[v.rating]}`}>{v.rating.replace("-", " ")}</div>
              </div>
            ))}
          </div>
        )}
      </section>

      {/* API Latency */}
      <section>
        <h2 className="text-lg font-semibold mb-4">API Latency</h2>
        {apiLatencies.length === 0 ? (
          <p className="text-muted-foreground text-sm">No API calls tracked yet.</p>
        ) : (
          <div className="glass-card rounded-2xl p-6">
            <ResponsiveContainer width="100%" height={220}>
              <BarChart data={apiLatencies} layout="vertical" margin={{ left: 16, right: 16 }}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.05)" />
                <XAxis type="number" unit="ms" tick={{ fontSize: 11 }} />
                <YAxis type="category" dataKey="endpoint" tick={{ fontSize: 11 }} width={140} />
                <Tooltip formatter={(v: number) => [`${v}ms`, "Avg latency"]} />
                <Bar dataKey="latency" fill="rgb(99,102,241)" radius={[0, 4, 4, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        )}
      </section>

      {/* Error Rate */}
      <section>
        <h2 className="text-lg font-semibold mb-4">
          Errors{" "}
          <span className="text-sm font-normal text-muted-foreground">({errorCount} total)</span>
        </h2>
        {errorTimeline.length === 0 ? (
          <p className="text-muted-foreground text-sm">No errors recorded.</p>
        ) : (
          <div className="glass-card rounded-2xl p-6">
            <ResponsiveContainer width="100%" height={180}>
              <LineChart data={errorTimeline}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.05)" />
                <XAxis dataKey="time" tick={{ fontSize: 11 }} />
                <YAxis allowDecimals={false} tick={{ fontSize: 11 }} />
                <Tooltip />
                <Line type="monotone" dataKey="count" stroke="rgb(239,68,68)" dot={false} strokeWidth={2} />
              </LineChart>
            </ResponsiveContainer>
          </div>
        )}
      </section>
    </div>
  );
}
