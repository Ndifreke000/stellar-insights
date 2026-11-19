"use client";

import React from "react";
import { Clock } from "lucide-react";

export interface FreshnessBadgeProps {
  /** ISO string, timestamp number, or Date instance */
  timestamp?: string | Date | number | null;
  /** Label prefix displayed before timestamp */
  label?: string;
  /** Optional source description e.g. "Horizon", "Cache", "RPC" */
  source?: string;
  /** Additional custom class names */
  className?: string;
}

const ISO_TIMESTAMP = /^\d{4}-\d{2}-\d{2}T/;

export const FreshnessBadge: React.FC<FreshnessBadgeProps> = ({
  timestamp,
  label = "Data as of",
  source,
  className = "",
}) => {
  const formattedTime = React.useMemo(() => {
    if (timestamp === null || timestamp === undefined || timestamp === "") return "Real-time sync";
    // Pre-formatted text (e.g. "Just now", "2026-10-17 14:30:00 UTC") is shown as-is;
    // only machine timestamps (ISO strings, epoch numbers, Dates) are localised.
    if (typeof timestamp === "string" && !ISO_TIMESTAMP.test(timestamp)) return timestamp;
    const date = new Date(timestamp);
    if (isNaN(date.getTime())) return "Real-time sync";
    return date.toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  }, [timestamp]);

  const tooltip = React.useMemo(() => {
    if (timestamp === null || timestamp === undefined || timestamp === "") return undefined;
    if (typeof timestamp === "string") return `Data snapshot timestamp: ${timestamp}`;
    const date = new Date(timestamp);
    return isNaN(date.getTime()) ? undefined : `Data snapshot timestamp: ${date.toISOString()}`;
  }, [timestamp]);

  return (
    <div
      data-testid="freshness-badge"
      title={tooltip}
      className={`inline-flex items-center gap-1.5 text-[10px] font-mono text-muted-foreground/75 bg-slate-100/60 dark:bg-slate-800/60 px-2 py-0.5 rounded-md border border-border/40 ${className}`}
    >
      <Clock className="w-3 h-3 text-muted-foreground/60 shrink-0" aria-hidden="true" />
      <span>
        {label} <span className="font-semibold text-foreground/90">{formattedTime}</span>
      </span>
      {source && <span className="opacity-60 text-[9px]">({source})</span>}
    </div>
  );
};
