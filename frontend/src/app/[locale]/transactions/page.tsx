"use client";

/**
 * Virtualised, streaming Transactions page (Issue #2835).
 *
 * Uses cursor-paginated `useInfiniteTransactions` for server-driven sorting and
 * filtering, subscribes to live payment updates via the shared realtime client
 * (#2834), and renders rows through TanStack Virtual (`VirtualizedTransactionTable`)
 * so 10,000+ transactions scroll smoothly at 60fps.
 */
import React, { useCallback, useMemo, useState } from "react";
import { Activity, AlertCircle, Hexagon, RefreshCw, Search } from "lucide-react";
import { Link } from "@/i18n/navigation";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { Skeleton } from "@/components/ui/Skeleton";
import { useInfiniteTransactions } from "@/lib/react-query/queries";
import type {
  StreamingTransaction,
  TransactionFilters,
} from "@/lib/api/transactions";
import {
  VirtualizedTransactionTable,
  type TransactionSortOrder,
} from "@/components/transactions/VirtualizedTransactionTable";
import { useWebSocket, type WsMessage } from "@/hooks/useWebSocket";
import { usePatchableRows } from "@/hooks/useStreamingTableData";

function TransactionsPageContent() {
  const [searchInput, setSearchInput] = useState("");
  const [assetCode, setAssetCode] = useState("");
  const [status, setStatus] = useState<"successful" | "failed" | "">("");
  const [sortOrder, setSortOrder] = useState<TransactionSortOrder>("desc");

  const queryFilters = useMemo<Omit<TransactionFilters, "cursor">>(
    () => ({
      limit: 50,
      asset_code: assetCode.trim() || undefined,
      status: status || undefined,
      search: searchInput.trim() || undefined,
      sort_order: sortOrder,
    }),
    [assetCode, status, searchInput, sortOrder],
  );

  const txQuery = useInfiniteTransactions(queryFilters);

  const baseTransactions = useMemo<StreamingTransaction[]>(() => {
    const pages = txQuery.data?.pages ?? [];
    const merged: StreamingTransaction[] = [];
    const seen = new Set<string>();
    for (const page of pages) {
      for (const tx of page.data ?? []) {
        if (seen.has(tx.id)) continue;
        seen.add(tx.id);
        merged.push(tx);
      }
    }
    return merged;
  }, [txQuery.data]);

  const {
    rows: transactions,
    upsertRow,
    patchRow,
    resetOverlays,
    lastPatchedId,
  } = usePatchableRows(baseTransactions);

  const handleRealtimeMessage = useCallback(
    (msg: WsMessage) => {
      if (msg.type === "new_payment") {
        const id = String(
          msg.id ??
            msg.transaction_hash ??
            `${msg.corridor_id ?? "tx"}-${msg.timestamp ?? Date.now()}`,
        );
        const row: StreamingTransaction = {
          id,
          paging_token: String(msg.paging_token ?? id),
          transaction_hash: String(msg.transaction_hash ?? id),
          source_account: String(msg.source_account ?? msg.from ?? "GUNKNOWN"),
          from: String(msg.from ?? msg.source_account ?? "GUNKNOWN"),
          to: String(msg.to ?? msg.destination_account ?? "GUNKNOWN"),
          amount: String(msg.amount ?? "0"),
          asset_type:
            String(msg.asset_code ?? "XLM").toUpperCase() === "XLM"
              ? "native"
              : "credit_alphanum4",
          asset_code: msg.asset_code ? String(msg.asset_code) : "XLM",
          transaction_successful: msg.successful !== false,
          created_at: String(msg.timestamp ?? new Date().toISOString()),
        };
        upsertRow(row);
      } else if (msg.type === "transaction_update" && typeof msg.id === "string") {
        patchRow(msg.id, (current) => ({
          ...current,
          transaction_successful:
            typeof msg.successful === "boolean"
              ? msg.successful
              : current.transaction_successful,
          amount:
            msg.amount !== undefined ? String(msg.amount) : current.amount,
        }));
      }
    },
    [upsertRow, patchRow],
  );

  // Shares the single tab-wide WebSocket singleton (#2834).
  const { isConnected } = useWebSocket(undefined, {
    channels: ["payments", "transactions"],
    onMessage: handleRealtimeMessage,
  });

  const handleLoadMore = useCallback(() => {
    if (txQuery.hasNextPage && !txQuery.isFetchingNextPage) {
      void txQuery.fetchNextPage();
    }
  }, [txQuery]);

  const handleSortOrderChange = useCallback(
    (nextOrder: TransactionSortOrder) => {
      setSortOrder(nextOrder);
      resetOverlays();
    },
    [resetOverlays],
  );

  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
      {/* Header */}
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
            Ledger Stream // Transactions
          </div>
          <h1 className="text-3xl md:text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
            <Activity className="w-8 h-8 text-accent" aria-hidden="true" />
            Live Transactions
          </h1>
          <p className="text-xs font-mono text-muted-foreground mt-1">
            Cursor-paginated Stellar payment stream with virtualised rows and live WebSocket patching.
          </p>
        </div>

        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => {
              resetOverlays();
              void txQuery.refetch();
            }}
            disabled={txQuery.isFetching}
            className="flex items-center gap-2 px-3.5 py-2 bg-slate-900/60 border border-border/50 rounded-xl text-[10px] font-mono font-bold uppercase tracking-widest text-foreground hover:border-accent/40 disabled:opacity-50 transition-all"
          >
            <RefreshCw
              className={`w-3.5 h-3.5 ${txQuery.isFetching ? "animate-spin" : ""}`}
              aria-hidden="true"
            />
            Refresh
          </button>
          <Link
            href="/transactions/builder"
            className="flex items-center gap-2 px-4 py-2 bg-indigo-500/15 border border-indigo-500/30 rounded-xl text-[10px] font-bold uppercase tracking-widest text-indigo-300 hover:bg-indigo-500 hover:text-white transition-all"
          >
            <Hexagon className="w-3.5 h-3.5" aria-hidden="true" />
            Multi-Sig Builder
          </Link>
        </div>
      </div>

      {/* Server-driven filter & sort bar */}
      <div className="grid grid-cols-1 md:grid-cols-12 gap-3">
        <div className="md:col-span-6 relative">
          <Search
            className="w-4 h-4 text-muted-foreground absolute left-3.5 top-1/2 -translate-y-1/2 pointer-events-none"
            aria-hidden="true"
          />
          <input
            type="search"
            value={searchInput}
            onChange={(e) => {
              setSearchInput(e.target.value);
              resetOverlays();
            }}
            placeholder="Filter by tx hash or account (server-side)..."
            aria-label="Search transactions"
            className="w-full pl-10 pr-4 py-2.5 bg-slate-900/50 border border-border/50 rounded-xl text-xs font-mono text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-accent/50"
          />
        </div>
        <div className="md:col-span-3">
          <input
            type="text"
            value={assetCode}
            onChange={(e) => {
              setAssetCode(e.target.value);
              resetOverlays();
            }}
            placeholder="Asset (e.g. USDC, XLM)"
            aria-label="Filter by asset code"
            className="w-full px-3.5 py-2.5 bg-slate-900/50 border border-border/50 rounded-xl text-xs font-mono text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-accent/50"
          />
        </div>
        <div className="md:col-span-3">
          <select
            value={status}
            onChange={(e) => {
              setStatus(e.target.value as "successful" | "failed" | "");
              resetOverlays();
            }}
            aria-label="Filter by status"
            className="w-full px-3.5 py-2.5 bg-slate-900/50 border border-border/50 rounded-xl text-xs font-mono uppercase text-foreground focus:outline-none focus:ring-2 focus:ring-accent/50 cursor-pointer"
          >
            <option value="">Status: All</option>
            <option value="successful">Status: Settled</option>
            <option value="failed">Status: Failed</option>
          </select>
        </div>
      </div>

      {/* Table / loading / error states */}
      {txQuery.isError && transactions.length === 0 ? (
        <div
          role="alert"
          className="p-6 glass-card rounded-2xl border border-red-500/30 bg-red-500/5 flex items-center justify-between gap-4"
        >
          <div className="flex items-center gap-3 text-sm text-red-300">
            <AlertCircle className="w-5 h-5 shrink-0" aria-hidden="true" />
            <span>
              Unable to load transactions from the backend. Retry when the RPC stream is reachable.
            </span>
          </div>
          <button
            type="button"
            onClick={() => void txQuery.refetch()}
            className="px-3.5 py-1.5 rounded-lg bg-red-500/20 text-xs font-mono uppercase text-red-200 hover:bg-red-500/30 transition-colors"
          >
            Retry
          </button>
        </div>
      ) : txQuery.isPending && transactions.length === 0 ? (
        <div className="space-y-3">
          <Skeleton className="h-12 w-full rounded-xl" />
          <Skeleton className="h-12 w-full rounded-xl" />
          <Skeleton className="h-12 w-full rounded-xl" />
          <Skeleton className="h-12 w-full rounded-xl" />
        </div>
      ) : transactions.length === 0 ? (
        <div className="py-16 flex flex-col items-center justify-center glass-card rounded-2xl border-dashed">
          <AlertCircle
            className="w-10 h-10 text-muted-foreground/30 mb-3"
            aria-hidden="true"
          />
          <p className="text-xs font-mono text-muted-foreground uppercase tracking-widest">
            No transactions match the current filters
          </p>
        </div>
      ) : (
        <VirtualizedTransactionTable
          rows={transactions}
          sortOrder={sortOrder}
          onSortOrderChange={handleSortOrderChange}
          hasNextPage={txQuery.hasNextPage}
          isFetchingNextPage={txQuery.isFetchingNextPage}
          onLoadMore={handleLoadMore}
          lastPatchedId={lastPatchedId}
          isLiveConnected={isConnected}
        />
      )}
    </div>
  );
}

export default function TransactionsPage() {
  return (
    <ErrorBoundary>
      <TransactionsPageContent />
    </ErrorBoundary>
  );
}
