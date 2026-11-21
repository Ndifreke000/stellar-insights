"use client";

/**
 * Normalized, patchable row store for virtualised streaming tables (Issue #2835).
 *
 * Keeps row data in an ordered array + ID index map so live WebSocket patches
 * replace ONLY the affected row object reference (`prevRow !== nextRow`) while
 * every untouched sibling row retains strict referential equality (`===`).
 * Paired with `React.memo` row & cell components, a live update triggers an
 * O(1) React commit (1 row) instead of re-rendering the entire table.
 */
import { useCallback, useMemo, useState } from "react";

export interface NormalizedRowState<T extends { id: string }> {
  rows: T[];
  indexById: Map<string, number>;
  /** Tracks the most recently patched row id & monotonic counter for visual feedback. */
  lastPatchedId: string | null;
  patchCount: number;
}

export function buildNormalizedRowState<T extends { id: string }>(
  initialRows: T[],
): NormalizedRowState<T> {
  const rows: T[] = [];
  const indexById = new Map<string, number>();

  for (const row of initialRows) {
    const existingIdx = indexById.get(row.id);
    if (existingIdx !== undefined) {
      rows[existingIdx] = row;
    } else {
      indexById.set(row.id, rows.length);
      rows.push(row);
    }
  }

  return {
    rows,
    indexById,
    lastPatchedId: null,
    patchCount: 0,
  };
}

/**
 * Patch a single row by `id`, returning the exact same state object if the row
 * does not exist or the patch produces no value changes. Untouched row elements
 * keep their exact object references.
 */
export function applySingleRowPatch<T extends { id: string }>(
  state: NormalizedRowState<T>,
  id: string,
  updater: (row: T) => T,
): NormalizedRowState<T> {
  const idx = state.indexById.get(id);
  if (idx === undefined) return state;

  const current = state.rows[idx];
  const updated = updater(current);
  if (updated === current) return state;

  // Shallow array copy preserves referential identity of every other row.
  const nextRows = state.rows.slice();
  nextRows[idx] = updated;

  return {
    rows: nextRows,
    indexById: state.indexById,
    lastPatchedId: id,
    patchCount: state.patchCount + 1,
  };
}

/**
 * Upsert a streaming row: if `row.id` already exists, patch it in place;
 * otherwise prepend (or append) it without recreating existing row objects.
 */
export function upsertNormalizedRow<T extends { id: string }>(
  state: NormalizedRowState<T>,
  row: T,
  position: "prepend" | "append" = "prepend",
  maxRows = 10_000,
): NormalizedRowState<T> {
  const existingIdx = state.indexById.get(row.id);
  if (existingIdx !== undefined) {
    return applySingleRowPatch(state, row.id, () => row);
  }

  const nextRows =
    position === "prepend"
      ? [row, ...state.rows].slice(0, maxRows)
      : [...state.rows, row].slice(0, maxRows);

  const indexById = new Map<string, number>();
  for (let i = 0; i < nextRows.length; i++) {
    indexById.set(nextRows[i].id, i);
  }

  return {
    rows: nextRows,
    indexById,
    lastPatchedId: row.id,
    patchCount: state.patchCount + 1,
  };
}

/**
 * React hook wrapping [`NormalizedRowState`] with live single-row patching and
 * base-list reconciliation (e.g. when a new cursor page arrives from React Query).
 */
export function usePatchableRows<T extends { id: string }>(
  sourceRows: T[],
  maxRows = 10_000,
) {
  // Store overlays keyed by row id so live WebSocket patches apply cleanly on
  // top of React Query pages without mutating or re-cloning untouched rows.
  const [patches, setPatches] = useState<Map<string, T>>(() => new Map());
  const [prepended, setPrepended] = useState<T[]>([]);
  const [lastPatchedId, setLastPatchedId] = useState<string | null>(null);
  const [patchCount, setPatchCount] = useState(0);

  const rows = useMemo<T[]>(() => {
    const seen = new Set<string>();
    const merged: T[] = [];

    for (const item of prepended) {
      if (seen.has(item.id)) continue;
      seen.add(item.id);
      merged.push(patches.get(item.id) ?? item);
    }

    for (const item of sourceRows) {
      if (seen.has(item.id)) continue;
      seen.add(item.id);
      merged.push(patches.get(item.id) ?? item);
      if (merged.length >= maxRows) break;
    }

    return merged;
  }, [prepended, sourceRows, patches, maxRows]);

  const patchRow = useCallback(
    (id: string, updater: (current: T) => T) => {
      setPatches((prev) => {
        const base =
          prev.get(id) ??
          prepended.find((r) => r.id === id) ??
          sourceRows.find((r) => r.id === id);
        if (!base) return prev;
        const nextRow = updater(base);
        if (nextRow === base) return prev;
        const nextMap = new Map(prev);
        nextMap.set(id, nextRow);
        return nextMap;
      });
      setLastPatchedId(id);
      setPatchCount((c) => c + 1);
    },
    [prepended, sourceRows],
  );

  const upsertRow = useCallback(
    (row: T) => {
      const existsInSource = sourceRows.some((r) => r.id === row.id);
      if (existsInSource) {
        setPatches((prev) => {
          const nextMap = new Map(prev);
          nextMap.set(row.id, row);
          return nextMap;
        });
      } else {
        setPrepended((prev) => {
          const existingIdx = prev.findIndex((r) => r.id === row.id);
          if (existingIdx !== -1) {
            const copy = prev.slice();
            copy[existingIdx] = row;
            return copy;
          }
          return [row, ...prev].slice(0, maxRows);
        });
      }
      setLastPatchedId(row.id);
      setPatchCount((c) => c + 1);
    },
    [sourceRows, maxRows],
  );

  const resetOverlays = useCallback(() => {
    setPatches(new Map());
    setPrepended([]);
    setLastPatchedId(null);
  }, []);

  return {
    rows,
    patchRow,
    upsertRow,
    resetOverlays,
    lastPatchedId,
    patchCount,
  };
}
