"use client";

import React, { useMemo } from "react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ReferenceLine,
  ResponsiveContainer,
} from "recharts";
import { TrendingDown, TrendingUp, AlertTriangle, CheckCircle2 } from "lucide-react";
import { ChartExportButton } from "@/components/charts/ChartExportButton";
import { useChartExport } from "@/hooks/useChartExport";

export interface ForecastDataPoint {
  date: string;
  /** Actual health score (0-100) — null for future points */
  actual: number | null;
  /** Forecasted health score */
  forecast: number;
  /** Lower bound of confidence interval */
  lower: number;
  /** Upper bound of confidence interval */
  upper: number;
}

export interface CorridorForecast {
  corridorId: string;
  corridorLabel: string;
  points: ForecastDataPoint[];
  /** True if model predicts degradation within the forecast window */
  degradationAlert: boolean;
  /** Predicted health at end of forecast window */
  predictedHealth: number;
  /** Trend direction inferred from forecast */
  trend: "up" | "down" | "stable";
}

interface CorridorHealthForecastChartProps {
  forecast: CorridorForecast;
  className?: string;
}

// Simple exponential-smoothing forecaster for client-side preview.
// In production the backend ML service drives this data.
export function generateForecast(
  history: { date: string; health: number }[],
  corridorId: string,
  corridorLabel: string,
  forecastDays = 7
): CorridorForecast {
  const alpha = 0.3; // smoothing factor
  const beta = 0.1; // trend smoothing factor

  if (history.length < 2) {
    const fallback = history[0]?.health ?? 80;
    const points: ForecastDataPoint[] = history.map((h) => ({
      date: h.date,
      actual: h.health,
      forecast: h.health,
      lower: Math.max(0, h.health - 5),
      upper: Math.min(100, h.health + 5),
    }));
    for (let i = 1; i <= forecastDays; i++) {
      const d = new Date();
      d.setDate(d.getDate() + i);
      points.push({
        date: d.toISOString().split("T")[0],
        actual: null,
        forecast: fallback,
        lower: Math.max(0, fallback - 10),
        upper: Math.min(100, fallback + 10),
      });
    }
    return {
      corridorId,
      corridorLabel,
      points,
      degradationAlert: false,
      predictedHealth: fallback,
      trend: "stable",
    };
  }

  // Double exponential smoothing (Holt's method)
  let level = history[0].health;
  let trendVal = history[1].health - history[0].health;

  const actualPoints: ForecastDataPoint[] = history.map((h, i) => {
    if (i > 0) {
      const prevLevel = level;
      level = alpha * h.health + (1 - alpha) * (level + trendVal);
      trendVal = beta * (level - prevLevel) + (1 - beta) * trendVal;
    }
    return {
      date: h.date,
      actual: h.health,
      forecast: Math.round((level + trendVal) * 10) / 10,
      lower: Math.max(0, Math.round((level + trendVal - 5) * 10) / 10),
      upper: Math.min(100, Math.round((level + trendVal + 5) * 10) / 10),
    };
  });

  // Project forward
  const forecastPoints: ForecastDataPoint[] = [];
  for (let i = 1; i <= forecastDays; i++) {
    const projectedHealth = level + trendVal * i;
    const uncertainty = 3 + i * 1.5; // widening uncertainty
    const d = new Date();
    d.setDate(d.getDate() + i);
    forecastPoints.push({
      date: d.toISOString().split("T")[0],
      actual: null,
      forecast: Math.max(0, Math.min(100, Math.round(projectedHealth * 10) / 10)),
      lower: Math.max(0, Math.round((projectedHealth - uncertainty) * 10) / 10),
      upper: Math.min(100, Math.round((projectedHealth + uncertainty) * 10) / 10),
    });
  }

  const predictedHealth = forecastPoints[forecastPoints.length - 1]?.forecast ?? level;
  const degradationAlert = predictedHealth < 70 || trendVal < -1.5;
  const trend: "up" | "down" | "stable" =
    trendVal > 0.5 ? "up" : trendVal < -0.5 ? "down" : "stable";

  return {
    corridorId,
    corridorLabel,
    points: [...actualPoints, ...forecastPoints],
    degradationAlert,
    predictedHealth,
    trend,
  };
}

const CustomTooltip = ({
  active,
  payload,
  label,
}: {
  active?: boolean;
  payload?: Array<{ name: string; value: number; color: string }>;
  label?: string;
}) => {
  if (!active || !payload?.length) return null;

  return (
    <div className="glass rounded-lg px-3 py-2 border border-white/10 text-[10px] font-mono space-y-1">
      <p className="text-muted-foreground uppercase tracking-widest">{label}</p>
      {payload.map((entry) => (
        <p key={entry.name} style={{ color: entry.color }}>
          {entry.name}: {typeof entry.value === "number" ? entry.value.toFixed(1) : "—"}
        </p>
      ))}
    </div>
  );
};

export function CorridorHealthForecastChart({
  forecast,
  className = "",
}: CorridorHealthForecastChartProps) {
  const { chartRef, isExporting, handleExport } = useChartExport({
    chartName: `${forecast.corridorLabel}-health-forecast`,
  });

  const today = new Date().toISOString().split("T")[0];

  const TrendIcon =
    forecast.trend === "up"
      ? TrendingUp
      : forecast.trend === "down"
      ? TrendingDown
      : null;

  const trendColor =
    forecast.trend === "up"
      ? "text-green-400"
      : forecast.trend === "down"
      ? "text-red-400"
      : "text-yellow-400";

  return (
    <div className={`glass-card rounded-2xl p-6 space-y-4 ${className}`}>
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-start justify-between gap-3">
        <div className="space-y-1">
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em]">
            Health Forecast // 7-Day
          </div>
          <h3 className="text-lg font-black tracking-tighter uppercase">
            {forecast.corridorLabel}
          </h3>
          <div className={`flex items-center gap-1.5 text-[11px] font-mono ${trendColor}`}>
            {TrendIcon && <TrendIcon className="w-3.5 h-3.5" aria-hidden="true" />}
            <span>
              Predicted: {forecast.predictedHealth.toFixed(0)}/100 •{" "}
              {forecast.trend === "up"
                ? "Improving"
                : forecast.trend === "down"
                ? "Degrading"
                : "Stable"}
            </span>
          </div>
        </div>

        <div className="flex items-center gap-2 flex-wrap">
          {forecast.degradationAlert ? (
            <div className="flex items-center gap-1.5 px-3 py-1.5 bg-red-500/10 border border-red-500/30 rounded-lg text-[10px] font-mono text-red-400 uppercase tracking-wider">
              <AlertTriangle className="w-3 h-3" aria-hidden="true" />
              Degradation Alert
            </div>
          ) : (
            <div className="flex items-center gap-1.5 px-3 py-1.5 bg-green-500/10 border border-green-500/30 rounded-lg text-[10px] font-mono text-green-400 uppercase tracking-wider">
              <CheckCircle2 className="w-3 h-3" aria-hidden="true" />
              On Track
            </div>
          )}
          <ChartExportButton
            chartRef={chartRef}
            chartName={`${forecast.corridorLabel}-health-forecast`}
          />
        </div>
      </div>

      {/* Legend note */}
      <div className="flex flex-wrap gap-4 text-[9px] font-mono text-muted-foreground uppercase tracking-widest">
        <span className="flex items-center gap-1.5">
          <span className="w-4 h-0.5 bg-accent inline-block" />
          Actual
        </span>
        <span className="flex items-center gap-1.5">
          <span className="w-4 h-0.5 border-t-2 border-dashed border-blue-400 inline-block" />
          Forecast
        </span>
        <span className="flex items-center gap-1.5">
          <span className="w-4 h-2 bg-blue-400/10 border border-blue-400/20 inline-block rounded" />
          Confidence Band
        </span>
        <span className="flex items-center gap-1.5">
          <span className="w-4 h-0.5 border-t border-dashed border-white/20 inline-block" />
          Today
        </span>
      </div>

      {/* Chart */}
      <div ref={chartRef} className="h-64 w-full">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart
            data={forecast.points}
            margin={{ top: 8, right: 8, bottom: 0, left: -16 }}
          >
            <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.05)" />
            <XAxis
              dataKey="date"
              tick={{ fill: "rgba(255,255,255,0.4)", fontSize: 9, fontFamily: "monospace" }}
              tickLine={false}
              axisLine={{ stroke: "rgba(255,255,255,0.1)" }}
              tickFormatter={(v: string) => v.slice(5)} // show MM-DD
            />
            <YAxis
              domain={[0, 100]}
              tick={{ fill: "rgba(255,255,255,0.4)", fontSize: 9, fontFamily: "monospace" }}
              tickLine={false}
              axisLine={false}
            />
            <Tooltip content={<CustomTooltip />} />
            {/* Confidence band — upper */}
            <Line
              dataKey="upper"
              stroke="rgba(96,165,250,0.15)"
              strokeWidth={0}
              dot={false}
              name="Upper"
              legendType="none"
              connectNulls
            />
            {/* Confidence band — lower */}
            <Line
              dataKey="lower"
              stroke="rgba(96,165,250,0.15)"
              strokeWidth={0}
              dot={false}
              name="Lower"
              legendType="none"
              connectNulls
            />
            {/* Actual health */}
            <Line
              dataKey="actual"
              stroke="#6366f1"
              strokeWidth={2}
              dot={false}
              name="Actual"
              connectNulls={false}
            />
            {/* Forecast */}
            <Line
              dataKey="forecast"
              stroke="#60a5fa"
              strokeWidth={2}
              strokeDasharray="5 4"
              dot={false}
              name="Forecast"
              connectNulls
            />
            {/* Today marker */}
            <ReferenceLine
              x={today}
              stroke="rgba(255,255,255,0.2)"
              strokeDasharray="3 3"
              label={{ value: "Today", fill: "rgba(255,255,255,0.3)", fontSize: 8 }}
            />
            {/* Danger threshold */}
            <ReferenceLine
              y={70}
              stroke="rgba(239,68,68,0.3)"
              strokeDasharray="4 3"
              label={{ value: "Warn", fill: "rgba(239,68,68,0.5)", fontSize: 8 }}
            />
            <Legend
              wrapperStyle={{ fontSize: 9, fontFamily: "monospace", color: "rgba(255,255,255,0.4)" }}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
