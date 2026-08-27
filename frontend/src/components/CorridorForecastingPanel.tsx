"use client";

import React, { useMemo } from "react";
import dynamic from "next/dynamic";
import { TrendingDown, AlertTriangle, Activity } from "lucide-react";
import { CorridorMetrics } from "@/lib/api/corridors";
import { generateForecast } from "@/components/charts/CorridorHealthForecastChart";

const CorridorHealthForecastChart = dynamic(
  () =>
    import("@/components/charts/CorridorHealthForecastChart").then(
      (m) => ({ default: m.CorridorHealthForecastChart })
    ),
  { ssr: false }
);

interface CorridorForecastingPanelProps {
  corridors: CorridorMetrics[];
  /** Maximum number of corridors to show forecasts for */
  maxCorridors?: number;
}

/**
 * Generates mock historical health data for a corridor and renders forecasts.
 * In production the backend `/api/corridors/:id/forecast` endpoint supplies
 * the history; here we derive it from the corridor's current health_score.
 */
function mockHistory(
  corridor: CorridorMetrics,
  days = 14
): { date: string; health: number }[] {
  const seed = corridor.health_score;
  const history: { date: string; health: number }[] = [];
  let h = seed;
  for (let i = days; i >= 0; i--) {
    const d = new Date();
    d.setDate(d.getDate() - i);
    // Simulate slight random-walk around the seed value
    h = Math.max(20, Math.min(100, h + (Math.random() - 0.5) * 4));
    history.push({ date: d.toISOString().split("T")[0], health: Math.round(h * 10) / 10 });
  }
  return history;
}

export function CorridorForecastingPanel({
  corridors,
  maxCorridors = 3,
}: CorridorForecastingPanelProps) {
  const forecasts = useMemo(
    () =>
      corridors.slice(0, maxCorridors).map((c) => {
        const history = mockHistory(c);
        return generateForecast(
          history,
          c.id,
          `${c.source_asset} → ${c.destination_asset}`,
          7
        );
      }),
    [corridors, maxCorridors]
  );

  const alertCount = forecasts.filter((f) => f.degradationAlert).length;

  return (
    <div className="space-y-6">
      {/* Section header */}
      <div className="flex flex-col sm:flex-row sm:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
            Corridor Intelligence // ML Forecasting
          </div>
          <h2 className="text-3xl font-black tracking-tighter uppercase italic flex items-center gap-3">
            <Activity className="w-7 h-7 text-accent" aria-hidden="true" />
            Health Forecasting
          </h2>
          <p className="text-sm text-muted-foreground mt-1 max-w-lg">
            Time-series forecasting predicts corridor health trends over the
            next 7 days. Alerts fire when predicted health drops below 70.
          </p>
        </div>

        {alertCount > 0 && (
          <div className="flex items-center gap-2 px-4 py-2 bg-red-500/10 border border-red-500/30 rounded-xl text-sm font-mono text-red-400">
            <AlertTriangle className="w-4 h-4 shrink-0" aria-hidden="true" />
            {alertCount} corridor{alertCount !== 1 ? "s" : ""} at risk
          </div>
        )}
      </div>

      {/* Forecast cards */}
      {forecasts.length === 0 ? (
        <div className="glass-card rounded-2xl p-8 text-center text-muted-foreground text-sm">
          No corridor data available to forecast.
        </div>
      ) : (
        <div className="grid grid-cols-1 xl:grid-cols-2 gap-6">
          {forecasts.map((forecast) => (
            <CorridorHealthForecastChart
              key={forecast.corridorId}
              forecast={forecast}
            />
          ))}
        </div>
      )}

      {/* Degradation alerts summary */}
      {alertCount > 0 && (
        <div className="glass-card rounded-2xl p-4 border border-red-500/20 space-y-3">
          <div className="flex items-center gap-2 text-[10px] font-mono uppercase tracking-widest text-red-400">
            <TrendingDown className="w-3.5 h-3.5" aria-hidden="true" />
            Degradation Alerts
          </div>
          <ul role="list" className="space-y-2">
            {forecasts
              .filter((f) => f.degradationAlert)
              .map((f) => (
                <li
                  key={f.corridorId}
                  className="flex items-center justify-between text-sm"
                >
                  <span className="font-mono text-foreground">{f.corridorLabel}</span>
                  <span className="font-mono text-red-400 text-xs">
                    Predicted: {f.predictedHealth.toFixed(0)}/100
                  </span>
                </li>
              ))}
          </ul>
          <p className="text-[10px] font-mono text-muted-foreground">
            Review these corridors and consider adjusting routing rules or
            contacting anchor operators.
          </p>
        </div>
      )}
    </div>
  );
}
