"use client";

import { useEffect } from "react";
import { AlertTriangle, RefreshCw } from "lucide-react";
import { logger } from "@/lib/logger";

interface ErrorProps {
  error: Error & { digest?: string };
  reset: () => void;
}

/**
 * Next.js App Router error boundary for the Governance page.
 * Catches runtime errors thrown inside this route segment and renders a
 * graceful fallback instead of crashing the entire app (#2331).
 */
export default function GovernanceError({ error, reset }: ErrorProps) {
  useEffect(() => {
    logger.error("Governance page error:", error);
  }, [error]);

  return (
    <div className="flex flex-col items-center justify-center min-h-[60vh] space-y-6 p-8">
      <div className="glass-card rounded-3xl border border-red-500/30 p-10 max-w-lg w-full text-center space-y-6">
        <div className="flex justify-center">
          <AlertTriangle className="w-14 h-14 text-red-500" />
        </div>
        <div>
          <div className="text-[10px] font-mono text-red-500 uppercase tracking-[0.2em] mb-2">
            Governance // Error
          </div>
          <h2 className="text-2xl font-black tracking-tighter uppercase italic mb-2">
            Governance Unavailable
          </h2>
          <p className="text-sm text-muted-foreground font-mono">
            An unexpected error occurred while loading governance proposals. Your data is safe.
          </p>
          {process.env.NODE_ENV === "development" && error?.message && (
            <pre className="mt-4 text-xs text-red-400/70 text-left bg-slate-900/50 p-3 rounded-xl overflow-auto max-h-32">
              {error.message}
            </pre>
          )}
        </div>
        <button
          onClick={reset}
          className="flex items-center justify-center gap-2 mx-auto px-6 py-3 bg-accent/10 border border-accent/30 rounded-xl text-[10px] font-bold uppercase tracking-widest text-accent hover:bg-accent hover:text-white transition-all"
        >
          <RefreshCw className="w-3 h-3" />
          Retry
        </button>
      </div>
    </div>
  );
}
