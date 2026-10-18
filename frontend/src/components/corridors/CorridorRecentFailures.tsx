"use client";

import { useEffect, useState, useCallback } from "react";
import {
  AlertCircle,
  ChevronLeft,
  ChevronRight,
  ExternalLink,
  RefreshCw,
  XOctagon,
  Copy,
  Check,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/Skeleton";
import {
  getCorridorFailures,
  type CorridorFailedPayment,
  type PaginatedCorridorFailures,
} from "@/lib/api/corridors";

interface CorridorRecentFailuresProps {
  corridorKey: string;
}

const RESULT_CODE_VARIANTS: Record<string, { bg: string; text: string; border: string }> = {
  op_underfunded: {
    bg: "bg-red-500/10 dark:bg-red-500/20",
    text: "text-red-700 dark:text-red-300",
    border: "border-red-500/30",
  },
  op_no_destination: {
    bg: "bg-rose-500/10 dark:bg-rose-500/20",
    text: "text-rose-700 dark:text-rose-300",
    border: "border-rose-500/30",
  },
  op_no_trust: {
    bg: "bg-purple-500/10 dark:bg-purple-500/20",
    text: "text-purple-700 dark:text-purple-300",
    border: "border-purple-500/30",
  },
  op_cross_failed: {
    bg: "bg-amber-500/10 dark:bg-amber-500/20",
    text: "text-amber-700 dark:text-amber-300",
    border: "border-amber-500/30",
  },
  tx_too_late: {
    bg: "bg-indigo-500/10 dark:bg-indigo-500/20",
    text: "text-indigo-700 dark:text-indigo-300",
    border: "border-indigo-500/30",
  },
  op_line_full: {
    bg: "bg-pink-500/10 dark:bg-pink-500/20",
    text: "text-pink-700 dark:text-pink-300",
    border: "border-pink-500/30",
  },
};

export function CorridorRecentFailures({ corridorKey }: CorridorRecentFailuresProps) {
  const [data, setData] = useState<PaginatedCorridorFailures | null>(null);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(5);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const fetchFailures = useCallback(async (targetPage: number) => {
    setLoading(true);
    setError(null);
    try {
      const res = await getCorridorFailures(corridorKey, targetPage, pageSize);
      setData(res);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load failure records");
    } finally {
      setLoading(false);
    }
  }, [corridorKey, pageSize]);

  useEffect(() => {
    setPage(1);
    fetchFailures(1);
  }, [fetchFailures]);

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    fetchFailures(newPage);
  };

  const copyTxHash = (hash: string, id: string) => {
    navigator.clipboard.writeText(hash);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  return (
    <div className="bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-xl p-6 shadow-sm mb-8 space-y-4">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 pb-3 border-b border-gray-100 dark:border-slate-700">
        <div className="flex items-center gap-2">
          <XOctagon className="w-5 h-5 text-red-500 shrink-0" />
          <h3 className="text-lg font-bold text-gray-900 dark:text-white">
            Recent Failed Payments
          </h3>
          {data && data.total > 0 && (
            <Badge variant="outline" className="border-red-500/30 text-red-500 font-mono text-xs">
              {data.total} recorded
            </Badge>
          )}
        </div>
        <button
          onClick={() => fetchFailures(page)}
          disabled={loading}
          className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1.5 self-start sm:self-auto transition-colors"
          aria-label="Refresh failures list"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${loading ? "animate-spin" : ""}`} />
          Refresh
        </button>
      </div>

      {/* Loading State */}
      {loading && !data ? (
        <div className="space-y-3 py-4">
          <Skeleton className="h-10 w-full rounded-lg" />
          <Skeleton className="h-10 w-full rounded-lg" />
          <Skeleton className="h-10 w-full rounded-lg" />
        </div>
      ) : error ? (
        /* Error State */
        <div className="p-4 bg-red-50 dark:bg-red-950/20 border border-red-200 dark:border-red-800 rounded-lg text-red-700 dark:text-red-300 flex items-center justify-between">
          <div className="flex items-center gap-2 text-sm">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span>{error}</span>
          </div>
          <button
            onClick={() => fetchFailures(page)}
            className="text-xs font-semibold underline hover:no-underline ml-4"
          >
            Retry
          </button>
        </div>
      ) : !data || data.items.length === 0 ? (
        /* Empty State */
        <div
          data-testid="failures-empty-state"
          className="text-center py-8 px-4 bg-gray-50/50 dark:bg-slate-900/30 rounded-lg border border-dashed border-gray-200 dark:border-slate-700"
        >
          <p className="text-sm font-medium text-gray-800 dark:text-gray-200">
            No Recent Payment Failures
          </p>
          <p className="text-xs text-muted-foreground mt-1 max-w-md mx-auto">
            All recent payments in this corridor were settled successfully. Failed payments will appear here with Stellar operation result codes when detected.
          </p>
        </div>
      ) : (
        /* Table of Failures */
        <div className="space-y-4">
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm" data-testid="failures-table">
              <thead>
                <tr className="border-b border-gray-100 dark:border-slate-700 text-xs font-mono uppercase text-muted-foreground">
                  <th className="py-2.5 px-3">Timestamp</th>
                  <th className="py-2.5 px-3">Result Code</th>
                  <th className="py-2.5 px-3">Category</th>
                  <th className="py-2.5 px-3">Amount</th>
                  <th className="py-2.5 px-3">Tx Hash</th>
                  <th className="py-2.5 px-3">Reason</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100 dark:divide-slate-800 font-mono text-xs">
                {data.items.map((fail: CorridorFailedPayment) => {
                  const style = RESULT_CODE_VARIANTS[fail.result_code] ?? {
                    bg: "bg-slate-500/10",
                    text: "text-slate-700 dark:text-slate-300",
                    border: "border-slate-500/30",
                  };
                  return (
                    <tr key={fail.id} className="hover:bg-gray-50/60 dark:hover:bg-slate-750/50 transition-colors">
                      <td className="py-3 px-3 whitespace-nowrap text-muted-foreground">
                        {new Date(fail.timestamp).toLocaleTimeString([], {
                          hour: "2-digit",
                          minute: "2-digit",
                          second: "2-digit",
                        })}
                      </td>
                      <td className="py-3 px-3 whitespace-nowrap">
                        <span
                          className={`inline-block px-2 py-0.5 rounded border text-[11px] font-bold ${style.bg} ${style.text} ${style.border}`}
                        >
                          {fail.result_code}
                        </span>
                      </td>
                      <td className="py-3 px-3 whitespace-nowrap text-muted-foreground capitalize">
                        {fail.category.replace(/_/g, " ")}
                      </td>
                      <td className="py-3 px-3 whitespace-nowrap font-bold text-gray-900 dark:text-white">
                        ${fail.amount.toFixed(2)}
                      </td>
                      <td className="py-3 px-3 whitespace-nowrap">
                        <div className="flex items-center gap-1.5 text-muted-foreground">
                          <span>
                            {fail.transaction_hash.slice(0, 6)}...{fail.transaction_hash.slice(-4)}
                          </span>
                          <button
                            onClick={() => copyTxHash(fail.transaction_hash, fail.id)}
                            className="p-1 hover:text-foreground transition-colors"
                            title="Copy transaction hash"
                            aria-label="Copy transaction hash"
                          >
                            {copiedId === fail.id ? (
                              <Check className="w-3 h-3 text-emerald-500" />
                            ) : (
                              <Copy className="w-3 h-3" />
                            )}
                          </button>
                        </div>
                      </td>
                      <td className="py-3 px-3 max-w-xs truncate text-muted-foreground" title={fail.error_message}>
                        {fail.error_message}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          {/* Pagination Controls */}
          {data.total_pages > 1 && (
            <div className="flex items-center justify-between pt-2 text-xs font-mono text-muted-foreground">
              <span>
                Page {data.page} of {data.total_pages} ({data.total} total)
              </span>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => handlePageChange(data.page - 1)}
                  disabled={data.page <= 1 || loading}
                  className="px-3 py-1.5 rounded-lg border border-border/50 hover:bg-slate-100 dark:hover:bg-slate-700 disabled:opacity-40 transition-colors flex items-center gap-1"
                  aria-label="Previous page"
                >
                  <ChevronLeft className="w-3.5 h-3.5" />
                  Prev
                </button>
                <button
                  onClick={() => handlePageChange(data.page + 1)}
                  disabled={data.page >= data.total_pages || loading}
                  className="px-3 py-1.5 rounded-lg border border-border/50 hover:bg-slate-100 dark:hover:bg-slate-700 disabled:opacity-40 transition-colors flex items-center gap-1"
                  aria-label="Next page"
                >
                  Next
                  <ChevronRight className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
