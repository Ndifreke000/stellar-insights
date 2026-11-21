"use client";

/**
 * Virtualised, streaming transaction table powered by TanStack Virtual (Issue #2835).
 *
 * - Virtualises 10,000+ rows at 60fps with fixed-height row recycling
 * - Stable row keys (`getItemKey: (i) => rows[i].id`)
 * - Memoized row and cell components so live WebSocket patches touch ONLY the
 *   affected row
 * - Server-driven sorting/filtering with cursor pagination (`next_cursor`)
 */
import React, { memo, useCallback, useEffect, useRef, useState } from "react";
import { observeElementRect, useVirtualizer } from "@tanstack/react-virtual";
import {
  ArrowDown,
  ArrowUp,
  Check,
  CheckCircle2,
  Copy,
  Loader2,
  Radio,
  XCircle,
} from "lucide-react";
import type { StreamingTransaction } from "@/lib/api/transactions";

export type TransactionSortOrder = "desc" | "asc";

export interface VirtualizedTransactionTableProps {
  rows: StreamingTransaction[];
  sortOrder: TransactionSortOrder;
  onSortOrderChange?: (order: TransactionSortOrder) => void;
  hasNextPage?: boolean;
  isFetchingNextPage?: boolean;
  onLoadMore?: () => void;
  lastPatchedId?: string | null;
  isLiveConnected?: boolean;
  height?: number;
  rowHeight?: number;
  /** Optional hook for React Profiler / render-count verification in tests. */
  onRowRender?: (txId: string) => void;
}

const AMOUNT_FORMAT = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 4,
});

function truncateMiddle(value: string, head = 6, tail = 4): string {
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}…${value.slice(-tail)}`;
}

function formatIsoTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toISOString().replace("T", " ").slice(0, 19) + "Z";
}

// ── Memoized Cells ────────────────────────────────────────────────────────────

const TxTimeCell = memo(function TxTimeCell({
  createdAt,
}: {
  createdAt: string;
}) {
  return (
    <div
      role="cell"
      className="flex-[1.3] px-3 text-xs font-mono text-muted-foreground whitespace-nowrap"
    >
      {formatIsoTime(createdAt)}
    </div>
  );
});

const TxHashCell = memo(function TxHashCell({ hash }: { hash: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    if (typeof navigator !== "undefined" && navigator.clipboard) {
      void navigator.clipboard.writeText(hash);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }
  }, [hash]);

  return (
    <div
      role="cell"
      className="flex-[1.2] px-3 text-xs font-mono text-foreground flex items-center gap-1.5"
    >
      <span title={hash}>{truncateMiddle(hash, 8, 6)}</span>
      <button
        type="button"
        onClick={handleCopy}
        aria-label="Copy transaction hash"
        className="p-1 text-muted-foreground hover:text-foreground transition-colors"
      >
        {copied ? (
          <Check className="w-3 h-3 text-emerald-400" aria-hidden="true" />
        ) : (
          <Copy className="w-3 h-3" aria-hidden="true" />
        )}
      </button>
    </div>
  );
});

const TxAccountsCell = memo(function TxAccountsCell({
  from,
  to,
}: {
  from: string;
  to: string;
}) {
  return (
    <div
      role="cell"
      className="flex-[1.8] px-3 text-xs font-mono text-muted-foreground truncate"
      title={`${from} → ${to}`}
    >
      <span className="text-foreground/90">{truncateMiddle(from, 6, 4)}</span>
      <span className="mx-1.5 text-accent">→</span>
      <span className="text-foreground/90">{truncateMiddle(to, 6, 4)}</span>
    </div>
  );
});

const TxAmountCell = memo(function TxAmountCell({
  amount,
  assetCode,
  assetType,
}: {
  amount: string;
  assetCode?: string | null;
  assetType: string;
}) {
  const numeric = Number.parseFloat(amount);
  const formatted = Number.isFinite(numeric)
    ? AMOUNT_FORMAT.format(numeric)
    : amount;
  const symbol =
    assetType === "native" ? "XLM" : (assetCode || "ASSET").toUpperCase();

  return (
    <div
      role="cell"
      className="flex-[1.1] px-3 text-xs font-mono font-bold text-foreground whitespace-nowrap"
    >
      {formatted}{" "}
      <span className="text-[10px] font-normal text-accent">{symbol}</span>
    </div>
  );
});

const TxStatusCell = memo(function TxStatusCell({
  successful,
}: {
  successful: boolean;
}) {
  return (
    <div role="cell" className="flex-[0.9] px-3 flex items-center">
      {successful ? (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md border border-emerald-500/20 bg-emerald-500/10 text-emerald-400 text-[11px] font-mono font-semibold">
          <CheckCircle2 className="w-3 h-3" aria-hidden="true" />
          Settled
        </span>
      ) : (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md border border-red-500/20 bg-red-500/10 text-red-400 text-[11px] font-mono font-semibold">
          <XCircle className="w-3 h-3" aria-hidden="true" />
          Failed
        </span>
      )}
    </div>
  );
});

// ── Memoized Virtual Row ──────────────────────────────────────────────────────

interface TransactionRowProps {
  tx: StreamingTransaction;
  virtualStart: number;
  rowHeight: number;
  rowIndex: number;
  isPatched: boolean;
  onRowRender?: (txId: string) => void;
}

const TransactionVirtualRow = memo(function TransactionVirtualRow({
  tx,
  virtualStart,
  rowHeight,
  rowIndex,
  isPatched,
  onRowRender,
}: TransactionRowProps) {
  onRowRender?.(tx.id);
  const successful = tx.transaction_successful !== false;

  return (
    <div
      role="row"
      aria-rowindex={rowIndex + 2}
      data-testid={`transaction-row-${tx.id}`}
      data-row-id={tx.id}
      data-patched={isPatched ? "true" : "false"}
      style={{
        position: "absolute",
        top: 0,
        left: 0,
        width: "100%",
        height: `${rowHeight}px`,
        transform: `translateY(${virtualStart}px)`,
      }}
      className={`flex items-center border-b border-border/30 transition-colors hover:bg-slate-800/40 ${
        isPatched
          ? "bg-accent/15 ring-1 ring-inset ring-accent/40"
          : rowIndex % 2 === 0
            ? "bg-slate-950/30"
            : "bg-slate-900/20"
      }`}
    >
      <TxTimeCell createdAt={tx.created_at} />
      <TxHashCell hash={tx.transaction_hash} />
      <TxAccountsCell from={tx.from || tx.source_account} to={tx.to} />
      <TxAmountCell
        amount={tx.amount}
        assetCode={tx.asset_code}
        assetType={tx.asset_type}
      />
      <TxStatusCell successful={successful} />
    </div>
  );
});

// ── Main Virtualised Transaction Table ────────────────────────────────────────

export function VirtualizedTransactionTable({
  rows,
  sortOrder,
  onSortOrderChange,
  hasNextPage = false,
  isFetchingNextPage = false,
  onLoadMore,
  lastPatchedId = null,
  isLiveConnected = false,
  height = 560,
  rowHeight = 52,
  onRowRender,
}: VirtualizedTransactionTableProps) {
  const scrollContainerRef = useRef<HTMLDivElement | null>(null);

  const getItemKey = useCallback(
    (index: number) => rows[index]?.id ?? String(index),
    [rows],
  );

  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollContainerRef.current,
    estimateSize: () => rowHeight,
    getItemKey,
    overscan: 10,
    initialRect: { width: 1000, height },
    observeElementRect: (instance, cb) =>
      observeElementRect(instance, (rect) => {
        cb(
          rect.height > 0
            ? rect
            : { width: rect.width || 1000, height },
        );
      }),
  });

  const virtualItems = virtualizer.getVirtualItems();
  const lastVisibleIndex =
    virtualItems.length > 0
      ? virtualItems[virtualItems.length - 1].index
      : -1;

  useEffect(() => {
    if (
      lastVisibleIndex >= rows.length - 10 &&
      hasNextPage &&
      !isFetchingNextPage &&
      onLoadMore
    ) {
      onLoadMore();
    }
  }, [
    lastVisibleIndex,
    rows.length,
    hasNextPage,
    isFetchingNextPage,
    onLoadMore,
  ]);

  const toggleSort = useCallback(() => {
    onSortOrderChange?.(sortOrder === "desc" ? "asc" : "desc");
  }, [onSortOrderChange, sortOrder]);

  return (
    <div
      className="glass-card rounded-2xl border border-border/50 overflow-hidden"
      data-testid="virtualized-transaction-table"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 px-4 py-2.5 border-b border-border/40 bg-slate-950/60 text-[11px] font-mono text-muted-foreground">
        <div className="flex items-center gap-2">
          <Radio
            className={`w-3.5 h-3.5 ${
              isLiveConnected ? "text-emerald-400 animate-pulse" : "text-slate-500"
            }`}
            aria-hidden="true"
          />
          <span>
            {isLiveConnected ? "LIVE STREAM" : "PAUSED"} ·{" "}
            <strong className="text-foreground">
              {rows.length.toLocaleString("en-US")}
            </strong>{" "}
            transactions ({virtualItems.length} rendered in DOM)
          </span>
        </div>
        {isFetchingNextPage && (
          <span className="inline-flex items-center gap-1.5 text-accent">
            <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" />
            Fetching cursor page…
          </span>
        )}
      </div>

      <div
        role="table"
        aria-label="Streaming transactions"
        aria-rowcount={rows.length + 1}
        className="w-full"
      >
        <div
          role="rowgroup"
          className="bg-slate-900/80 border-b border-border/50 text-[10px] font-mono uppercase tracking-wider text-muted-foreground"
        >
          <div role="row" aria-rowindex={1} className="flex items-center h-11">
            <div
              role="columnheader"
              aria-sort={sortOrder === "asc" ? "ascending" : "descending"}
              className="flex-[1.3] px-3"
            >
              <button
                type="button"
                onClick={toggleSort}
                aria-label="Sort transactions by timestamp"
                className="inline-flex items-center gap-1 uppercase hover:text-foreground transition-colors cursor-pointer"
              >
                <span>Timestamp</span>
                {sortOrder === "asc" ? (
                  <ArrowUp className="w-3 h-3 text-accent" aria-hidden="true" />
                ) : (
                  <ArrowDown className="w-3 h-3 text-accent" aria-hidden="true" />
                )}
              </button>
            </div>
            <div role="columnheader" className="flex-[1.2] px-3">
              Tx Hash
            </div>
            <div role="columnheader" className="flex-[1.8] px-3">
              Route (Source → Dest)
            </div>
            <div role="columnheader" className="flex-[1.1] px-3">
              Amount
            </div>
            <div role="columnheader" className="flex-[0.9] px-3">
              Status
            </div>
          </div>
        </div>

        <div
          ref={scrollContainerRef}
          data-testid="transaction-table-scroll-container"
          style={{ height: `${height}px` }}
          className="overflow-y-auto overflow-x-auto relative will-change-scroll"
        >
          <div
            role="rowgroup"
            style={{
              height: `${virtualizer.getTotalSize()}px`,
              width: "100%",
              position: "relative",
            }}
          >
            {virtualItems.map((virtualItem) => {
              const tx = rows[virtualItem.index];
              if (!tx) return null;
              return (
                <TransactionVirtualRow
                  key={tx.id}
                  tx={tx}
                  virtualStart={virtualItem.start}
                  rowHeight={rowHeight}
                  rowIndex={virtualItem.index}
                  isPatched={lastPatchedId === tx.id}
                  onRowRender={onRowRender}
                />
              );
            })}
          </div>
        </div>
      </div>

      {hasNextPage && onLoadMore && (
        <div className="flex items-center justify-center py-3 border-t border-border/40 bg-slate-950/50">
          <button
            type="button"
            onClick={onLoadMore}
            disabled={isFetchingNextPage}
            className="px-4 py-1.5 rounded-lg border border-accent/30 bg-accent/10 text-xs font-mono uppercase tracking-wider text-accent hover:bg-accent hover:text-white disabled:opacity-50 transition-colors"
          >
            {isFetchingNextPage ? "Loading more…" : "Load older transactions"}
          </button>
        </div>
      )}
    </div>
  );
}
