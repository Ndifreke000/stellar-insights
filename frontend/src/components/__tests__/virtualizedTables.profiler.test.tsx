import React, { Profiler, useEffect, useState } from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, act, fireEvent } from "@testing-library/react";
import { VirtualizedCorridorTable } from "@/components/corridors/VirtualizedCorridorTable";
import { VirtualizedTransactionTable } from "@/components/transactions/VirtualizedTransactionTable";
import {
  applySingleRowPatch,
  buildNormalizedRowState,
  upsertNormalizedRow,
  usePatchableRows,
} from "@/hooks/useStreamingTableData";
import type { CorridorMetrics } from "@/lib/api/corridors";
import type { StreamingTransaction } from "@/lib/api/transactions";

vi.mock("@/i18n/navigation", () => ({
  Link: ({
    href,
    children,
    ...props
  }: {
    href: string;
    children: React.ReactNode;
  }) => (
    <a href={href} {...props}>
      {children}
    </a>
  ),
}));

function makeCorridors(count: number): CorridorMetrics[] {
  const rows: CorridorMetrics[] = new Array(count);
  for (let i = 0; i < count; i++) {
    rows[i] = {
      id: `corridor-${String(i).padStart(5, "0")}`,
      source_asset: i % 2 === 0 ? "USDC" : "EURC",
      destination_asset: i % 3 === 0 ? "NGN" : i % 3 === 1 ? "PHP" : "KES",
      success_rate: 90 + (i % 10),
      total_attempts: 100 + i,
      successful_payments: 90 + i,
      failed_payments: 10,
      average_latency_ms: 350 + (i % 200),
      median_latency_ms: 300,
      p95_latency_ms: 800,
      p99_latency_ms: 1200,
      liquidity_depth_usd: 100_000 + i * 100,
      liquidity_volume_24h_usd: 10_000 + i * 10,
      liquidity_trend: "stable",
      average_slippage_bps: 12.5,
      health_score: 75 + (i % 25),
      last_updated: "2026-11-21T00:00:00Z",
    };
  }
  return rows;
}

function makeTransactions(count: number): StreamingTransaction[] {
  const rows: StreamingTransaction[] = new Array(count);
  for (let i = 0; i < count; i++) {
    const id = `tx-${String(i).padStart(5, "0")}`;
    rows[i] = {
      id,
      paging_token: String(1_000_000 - i),
      transaction_hash: `${id.replace("-", "")}abcdef0123456789abcdef0123456789abcdef`,
      source_account: "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
      from: "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
      to: "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
      amount: String((10 + (i % 500)).toFixed(2)),
      asset_type: i % 2 === 0 ? "credit_alphanum4" : "native",
      asset_code: i % 2 === 0 ? "USDC" : null,
      transaction_successful: i % 10 !== 0,
      created_at: "2026-11-21T12:00:00Z",
    };
  }
  return rows;
}

/** Baseline naive unvirtualized table representing the pre-#2835 implementation. */
function NaiveCorridorTable({ rows }: { rows: CorridorMetrics[] }) {
  return (
    <div role="table" data-testid="naive-corridor-table">
      {rows.map((c) => (
        <div key={c.id} role="row" data-row-id={c.id}>
          <span>
            {c.source_asset} → {c.destination_asset}
          </span>
          <span>{c.success_rate.toFixed(1)}%</span>
          <span>{c.health_score.toFixed(1)}</span>
        </div>
      ))}
    </div>
  );
}

describe("Virtualised streaming corridor & transaction tables (#2835)", () => {
  it("renders only the visible window for 10,000 corridors and patches ONLY the affected row", () => {
    const tenThousand = makeCorridors(10_000);
    const renderedIds: string[] = [];
    const trackRender = (id: string) => {
      renderedIds.push(id);
    };
    let triggerPatch: ((id: string, rate: number) => void) | null = null;
    const registerPatch = (fn: (id: string, rate: number) => void) => {
      triggerPatch = fn;
    };

    function Harness({
      onReady,
    }: {
      onReady: (fn: (id: string, rate: number) => void) => void;
    }) {
      const { rows, patchRow } = usePatchableRows(tenThousand);

      useEffect(() => {
        onReady((id: string, rate: number) => {
          patchRow(id, (cur) => ({ ...cur, success_rate: rate }));
        });
      }, [onReady, patchRow]);

      return (
        <VirtualizedCorridorTable
          rows={rows}
          sortBy="health_score"
          sortOrder="desc"
          isLiveConnected
          onRowRender={trackRender}
        />
      );
    }

    const { container } = render(<Harness onReady={registerPatch} />);

    const domRows = container.querySelectorAll("[data-row-id]");
    // 10,000 rows in dataset, but TanStack Virtual mounts only ~20-35 visible + overscan DOM rows!
    expect(domRows.length).toBeGreaterThan(5);
    expect(domRows.length).toBeLessThan(60);

    // Clear initial render tracking and apply a single-row live WebSocket patch
    // to row index 2 (visible inside the initial viewport).
    renderedIds.length = 0;

    act(() => {
      triggerPatch?.("corridor-00002", 99.9);
    });

    // CRITICAL INVARIANT: ONLY the single patched row re-rendered (O(1)),
    // NOT the other visible rows and NOT all 10,000 rows!
    expect(renderedIds).toEqual(["corridor-00002"]);
    expect(
      screen.getByTestId("corridor-row-corridor-00002"),
    ).toHaveTextContent("99.9%");
  });

  it("renders only the visible window for 10,000 transactions and patches ONLY the affected row", () => {
    const tenThousandTx = makeTransactions(10_000);
    const renderedIds: string[] = [];
    const trackTxRender = (id: string) => {
      renderedIds.push(id);
    };
    let triggerPatch: ((id: string, amount: string) => void) | null = null;
    const registerTxPatch = (fn: (id: string, amount: string) => void) => {
      triggerPatch = fn;
    };

    function TxHarness({
      onReady,
    }: {
      onReady: (fn: (id: string, amount: string) => void) => void;
    }) {
      const { rows, patchRow } = usePatchableRows(tenThousandTx);

      useEffect(() => {
        onReady((id: string, amount: string) => {
          patchRow(id, (cur) => ({ ...cur, amount, transaction_successful: true }));
        });
      }, [onReady, patchRow]);

      return (
        <VirtualizedTransactionTable
          rows={rows}
          sortOrder="desc"
          isLiveConnected
          onRowRender={trackTxRender}
        />
      );
    }

    const { container } = render(<TxHarness onReady={registerTxPatch} />);
    const domRows = container.querySelectorAll("[data-row-id]");
    expect(domRows.length).toBeGreaterThan(5);
    expect(domRows.length).toBeLessThan(60);

    renderedIds.length = 0;
    act(() => {
      triggerPatch?.("tx-00001", "9876.54");
    });

    expect(renderedIds).toEqual(["tx-00001"]);
    expect(screen.getByTestId("transaction-row-tx-00001")).toHaveTextContent(
      "9,876.54",
    );
  });

  it("React Profiler measurement: virtualised table drastically reduces mount & patch cost vs naive table", () => {
    const dataset = makeCorridors(2_000);
    const naiveMetrics: Array<{ phase: string; actualDuration: number }> = [];
    const virtualMetrics: Array<{ phase: string; actualDuration: number }> = [];

    let patchNaive: (() => void) | null = null;
    const registerNaive = (fn: () => void) => {
      patchNaive = fn;
    };

    function NaiveWrapper({ onReady }: { onReady: (fn: () => void) => void }) {
      const [rows, setRows] = useState(dataset);

      useEffect(() => {
        onReady(() =>
          setRows((prev) =>
            prev.map((r) =>
              r.id === "corridor-00003" ? { ...r, success_rate: 99.7 } : r,
            ),
          ),
        );
      }, [onReady]);

      return (
        <Profiler
          id="naive-corridors"
          onRender={(_id, phase, actualDuration) => {
            naiveMetrics.push({ phase, actualDuration });
          }}
        >
          <NaiveCorridorTable rows={rows} />
        </Profiler>
      );
    }

    let patchVirtual: (() => void) | null = null;
    const registerVirtual = (fn: () => void) => {
      patchVirtual = fn;
    };

    function VirtualWrapper({ onReady }: { onReady: (fn: () => void) => void }) {
      const { rows, patchRow } = usePatchableRows(dataset);

      useEffect(() => {
        onReady(() =>
          patchRow("corridor-00003", (cur) => ({ ...cur, success_rate: 99.7 })),
        );
      }, [onReady, patchRow]);

      return (
        <Profiler
          id="virtualized-corridors"
          onRender={(_id, phase, actualDuration) => {
            virtualMetrics.push({ phase, actualDuration });
          }}
        >
          <VirtualizedCorridorTable
            rows={rows}
            sortBy="health_score"
            sortOrder="desc"
          />
        </Profiler>
      );
    }

    const naiveRender = render(<NaiveWrapper onReady={registerNaive} />);
    const naiveDomCount =
      naiveRender.container.querySelectorAll("[data-row-id]").length;
    act(() => {
      patchNaive?.();
    });
    naiveRender.unmount();

    const virtualRender = render(<VirtualWrapper onReady={registerVirtual} />);
    const virtualDomCount =
      virtualRender.container.querySelectorAll("[data-row-id]").length;
    act(() => {
      patchVirtual?.();
    });
    virtualRender.unmount();

    const naiveMountMs = naiveMetrics.find((m) => m.phase === "mount")?.actualDuration ?? 0;
    const naivePatchMs = naiveMetrics.find((m) => m.phase === "update")?.actualDuration ?? 0;
    const virtualMountMs = virtualMetrics.find((m) => m.phase === "mount")?.actualDuration ?? 0;
    const virtualPatchMs = virtualMetrics.find((m) => m.phase === "update")?.actualDuration ?? 0;

    // Emit concrete profiler metrics for reproducible PR measurement evidence.
    console.info(
      `[React Profiler 2,000 rows] Naive: ${naiveDomCount} DOM rows, mount=${naiveMountMs.toFixed(2)}ms, 1-row patch=${naivePatchMs.toFixed(2)}ms | Virtualised: ${virtualDomCount} DOM rows, mount=${virtualMountMs.toFixed(2)}ms, 1-row patch=${virtualPatchMs.toFixed(2)}ms`,
    );

    expect(naiveDomCount).toBe(2_000);
    expect(virtualDomCount).toBeLessThan(60);
    expect(virtualMountMs).toBeLessThan(naiveMountMs);
    expect(virtualPatchMs).toBeLessThan(naivePatchMs);
  });

  it("emits server-driven sort changes and triggers cursor load-more callbacks", () => {
    const onSortChange = vi.fn();
    const onLoadMore = vi.fn();
    const sample = makeCorridors(5);

    render(
      <VirtualizedCorridorTable
        rows={sample}
        sortBy="health_score"
        sortOrder="desc"
        hasNextPage
        onSortChange={onSortChange}
        onLoadMore={onLoadMore}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Sort by Health" }));
    expect(onSortChange).toHaveBeenCalledWith("health_score", "asc");

    fireEvent.click(screen.getByRole("button", { name: "Sort by Success Rate" }));
    expect(onSortChange).toHaveBeenCalledWith("success_rate", "desc");

    // Near-tail effect automatically requests the next cursor page.
    expect(onLoadMore).toHaveBeenCalled();
  });

  it("normalized row store preserves referential identity on patch and deduplicates upserts", () => {
    const initial = makeCorridors(3);
    const state = buildNormalizedRowState(initial);

    // Unknown row id is a strict no-op returning the same state reference.
    const unchanged = applySingleRowPatch(state, "missing-id", (r) => ({
      ...r,
      success_rate: 1,
    }));
    expect(unchanged).toBe(state);

    // Patching row 1 preserves exact references of row 0 and row 2.
    const patched = applySingleRowPatch(state, initial[1].id, (r) => ({
      ...r,
      success_rate: 99.5,
    }));
    expect(patched.rows[1].success_rate).toBe(99.5);
    expect(patched.rows[0]).toBe(state.rows[0]);
    expect(patched.rows[2]).toBe(state.rows[2]);

    // Prepending a new row inserts at index 0; upserting an existing id updates in place without duplicating.
    const brandNew = { ...makeCorridors(1)[0], id: "corridor-new" };
    const withNew = upsertNormalizedRow(patched, brandNew, "prepend");
    expect(withNew.rows).toHaveLength(4);
    expect(withNew.rows[0].id).toBe("corridor-new");

    const updatedNew = upsertNormalizedRow(
      withNew,
      { ...brandNew, success_rate: 88.8 },
      "prepend",
    );
    expect(updatedNew.rows).toHaveLength(4);
    expect(updatedNew.rows[0].success_rate).toBe(88.8);
  });
});
