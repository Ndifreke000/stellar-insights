"use client";

import { AlertCircle, AlertTriangle } from "lucide-react";
import { Link } from "@/i18n/navigation";

interface ObservedDataNoticeProps {
  message: string;
  onRetry?: () => void;
  retrying?: boolean;
  /** Warning keeps already loaded observations on screen. Error replaces them. */
  variant?: "error" | "warning";
}

/**
 * Shown when live corridor data cannot be loaded. Never pairs with mock metrics.
 */
export function ObservedDataNotice({
  message,
  onRetry,
  retrying = false,
  variant = "error",
}: ObservedDataNoticeProps) {
  const isWarning = variant === "warning";
  const Icon = isWarning ? AlertTriangle : AlertCircle;

  return (
    <div
      role={isWarning ? "status" : "alert"}
      className={`flex flex-col sm:flex-row sm:items-center justify-between gap-3 p-4 rounded-xl text-sm border ${
        isWarning
          ? "bg-amber-500/10 border-amber-500/30 text-amber-300"
          : "bg-red-500/10 border-red-500/30 text-red-300"
      }`}
    >
      <div className="flex items-start gap-2">
        <Icon className="w-5 h-5 shrink-0 mt-0.5" aria-hidden="true" />
        <p>{message}</p>
      </div>
      <div className="flex items-center gap-2 shrink-0">
        {onRetry && (
          <button
            type="button"
            onClick={onRetry}
            disabled={retrying}
            className="px-3 py-1 rounded-md text-xs font-semibold uppercase tracking-wider border border-current/30 hover:bg-white/10 disabled:opacity-50"
          >
            Retry
          </button>
        )}
        <Link
          href="/status"
          className="px-3 py-1 rounded-md text-xs font-semibold uppercase tracking-wider underline underline-offset-2"
        >
          Status page
        </Link>
      </div>
    </div>
  );
}
