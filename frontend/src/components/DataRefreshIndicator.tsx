"use client";

import React from "react";

export interface DataRefreshIndicatorProps {
  /** Timestamp of the last successful data update. */
  lastUpdated: Date | null;
  /** Seconds remaining until the next auto-refresh. Must be >= 0. */
  secondsUntilRefresh: number;
  /** Total auto-refresh interval in seconds (used to calculate ring progress). */
  refreshIntervalSec?: number;
  /** True while a refresh is running. */
  isRefreshing: boolean;
  /** Called when the user clicks the manual refresh button. */
  onRefresh: () => void;
  className?: string;
}

/** SVG countdown ring radius and derived values. */
const RADIUS = 8;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

export function DataRefreshIndicator({
  lastUpdated,
  secondsUntilRefresh,
  refreshIntervalSec = 30,
  isRefreshing,
  onRefresh,
  className = "",
}: DataRefreshIndicatorProps) {
  // Progress fraction: 0 = just refreshed (full ring), 1 = about to refresh (empty)
  const progress =
    refreshIntervalSec > 0 ? 1 - secondsUntilRefresh / refreshIntervalSec : 0;
  const strokeDashoffset = CIRCUMFERENCE * progress;

  const formattedTime = lastUpdated
    ? lastUpdated.toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      })
    : "—";

  return (
    <div
      className={`flex items-center gap-2 flex-wrap ${className}`}
      aria-label="Data refresh status"
    >
      {/* ── Last updated badge ─────────────────────────────────── */}
      <div
        className="px-3 py-1.5 glass rounded-lg text-[10px] font-mono uppercase tracking-widest text-muted-foreground whitespace-nowrap"
        title={
          lastUpdated
            ? `Last updated: ${lastUpdated.toLocaleString()}`
            : "No data yet"
        }
      >
        Last Update:{" "}
        <span className="text-foreground font-semibold">{formattedTime}</span>
      </div>

      {/* ── Countdown ring badge ────────────────────────────────── */}
      <div
        className="flex items-center gap-1.5 px-3 py-1.5 glass rounded-lg text-[10px] font-mono uppercase tracking-widest text-muted-foreground whitespace-nowrap"
        title={`Auto-refresh in ${secondsUntilRefresh}s`}
      >
        {/* Animated circular progress ring */}
        <svg
          width="20"
          height="20"
          viewBox="0 0 20 20"
          className="shrink-0"
          aria-hidden="true"
        >
          {/* Track */}
          <circle
            cx="10"
            cy="10"
            r={RADIUS}
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            className="text-muted opacity-30"
          />
          {/* Progress arc (rotated so it starts at the top) */}
          <circle
            cx="10"
            cy="10"
            r={RADIUS}
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            strokeLinecap="round"
            strokeDasharray={CIRCUMFERENCE}
            strokeDashoffset={strokeDashoffset}
            className="text-accent transition-[stroke-dashoffset] duration-1000 ease-linear"
            style={{ transform: "rotate(-90deg)", transformOrigin: "50% 50%" }}
          />
        </svg>

        {isRefreshing ? (
          <span className="text-accent animate-pulse">Refreshing…</span>
        ) : (
          <>
            Next refresh in{" "}
            <span className="text-foreground font-semibold tabular-nums">
              {secondsUntilRefresh}s
            </span>
          </>
        )}
      </div>

      {/* ── Manual refresh button ───────────────────────────────── */}
      <button
        onClick={onRefresh}
        disabled={isRefreshing}
        aria-label="Manually refresh data"
        title="Refresh now"
        className={[
          "flex items-center gap-1.5 px-4 py-1.5",
          "bg-accent text-accent-foreground rounded-lg",
          "text-[10px] font-bold uppercase tracking-widest",
          "transition-all duration-200",
          "hover:scale-105 hover:shadow-[0_0_14px_var(--glow-accent-shadow)]",
          "active:scale-95",
          "disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100",
        ].join(" ")}
      >
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={`w-3 h-3 ${isRefreshing ? "animate-spin" : ""}`}
          aria-hidden="true"
        >
          <path d="M3 12a9 9 0 0 1 15-6.7L21 8" />
          <path d="M21 3v5h-5" />
          <path d="M21 12a9 9 0 0 1-15 6.7L3 16" />
          <path d="M3 21v-5h5" />
        </svg>
        {isRefreshing ? "Refreshing" : "Refresh"}
      </button>
    </div>
  );
}
