"use client";

/**
 * #2110 – Advanced Filtering and Search
 *
 * Generic, composable filter panel that can be dropped into any data table.
 * Supports text search, multi-value selects, numeric range sliders, and saved
 * search presets persisted to localStorage.
 */

import React, { useCallback, useEffect, useRef, useState } from "react";
import { Search, X, ChevronDown, Bookmark, BookmarkCheck, SlidersHorizontal } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

// ── Types ─────────────────────────────────────────────────────────────────────

export type FilterOperator = "eq" | "neq" | "gt" | "gte" | "lt" | "lte" | "contains" | "in";

export interface FilterField {
  /** Unique key matching the data row property */
  key: string;
  label: string;
  type: "text" | "select" | "number" | "range";
  /** Options for `select` type */
  options?: { value: string; label: string }[];
  /** Min/max for `range` type */
  min?: number;
  max?: number;
  step?: number;
  placeholder?: string;
}

export interface ActiveFilter {
  field: string;
  operator: FilterOperator;
  value: string | number | string[];
  label: string;
}

export interface SavedSearch {
  id: string;
  name: string;
  filters: ActiveFilter[];
  query: string;
  savedAt: string;
}

export interface AdvancedFilterState {
  query: string;
  filters: ActiveFilter[];
}

interface AdvancedFilterProps {
  fields: FilterField[];
  /** Called whenever query or filters change */
  onChange: (state: AdvancedFilterState) => void;
  /** localStorage key for saved searches (unique per page) */
  storageKey?: string;
  placeholder?: string;
  className?: string;
}

// ── Saved-search helpers ──────────────────────────────────────────────────────

function loadSavedSearches(key: string): SavedSearch[] {
  try {
    const raw = typeof window !== "undefined" ? localStorage.getItem(key) : null;
    return raw ? (JSON.parse(raw) as SavedSearch[]) : [];
  } catch {
    return [];
  }
}

function persistSavedSearches(key: string, searches: SavedSearch[]): void {
  try {
    localStorage.setItem(key, JSON.stringify(searches));
  } catch {
    // Ignore quota errors silently
  }
}

// ── Filter tag ────────────────────────────────────────────────────────────────

function FilterTag({
  filter,
  onRemove,
}: {
  filter: ActiveFilter;
  onRemove: () => void;
}) {
  const display =
    Array.isArray(filter.value)
      ? filter.value.join(", ")
      : String(filter.value);
  return (
    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-accent/20 border border-accent/30 text-[10px] font-mono text-accent">
      <span className="font-bold">{filter.label}:</span>
      <span className="max-w-[120px] truncate">{display}</span>
      <button
        onClick={onRemove}
        className="ml-0.5 hover:text-white transition-colors"
        aria-label={`Remove filter ${filter.label}`}
      >
        <X className="w-2.5 h-2.5" />
      </button>
    </span>
  );
}

// ── Single filter builder row ─────────────────────────────────────────────────

function FilterRow({
  fields,
  onAdd,
  onCancel,
}: {
  fields: FilterField[];
  onAdd: (f: ActiveFilter) => void;
  onCancel: () => void;
}) {
  const [selectedField, setSelectedField] = useState<FilterField | null>(null);
  const [operator, setOperator] = useState<FilterOperator>("contains");
  const [value, setValue] = useState<string>("");
  const [selectedOptions, setSelectedOptions] = useState<string[]>([]);

  const handleFieldChange = (key: string) => {
    const f = fields.find((x) => x.key === key) ?? null;
    setSelectedField(f);
    setValue("");
    setSelectedOptions([]);
    if (f?.type === "text") setOperator("contains");
    else if (f?.type === "select") setOperator("in");
    else setOperator("gte");
  };

  const handleAdd = () => {
    if (!selectedField) return;
    const v: ActiveFilter["value"] =
      selectedField.type === "select" ? selectedOptions : value;
    if (!v || (Array.isArray(v) && v.length === 0)) return;
    onAdd({ field: selectedField.key, operator, value: v, label: selectedField.label });
  };

  return (
    <div className="flex flex-wrap gap-2 items-end p-3 bg-white/5 rounded-xl border border-border/30">
      {/* Field selector */}
      <div className="flex flex-col gap-1 min-w-[140px]">
        <label className="text-[9px] font-mono uppercase text-muted-foreground">Field</label>
        <select
          className="bg-background border border-border/40 rounded-md px-2 py-1.5 text-xs font-mono focus:outline-none focus:ring-1 focus:ring-accent"
          value={selectedField?.key ?? ""}
          onChange={(e) => handleFieldChange(e.target.value)}
        >
          <option value="">— choose —</option>
          {fields.map((f) => (
            <option key={f.key} value={f.key}>
              {f.label}
            </option>
          ))}
        </select>
      </div>

      {/* Operator */}
      {selectedField && selectedField.type !== "select" && (
        <div className="flex flex-col gap-1 min-w-[120px]">
          <label className="text-[9px] font-mono uppercase text-muted-foreground">
            Operator
          </label>
          <select
            className="bg-background border border-border/40 rounded-md px-2 py-1.5 text-xs font-mono focus:outline-none focus:ring-1 focus:ring-accent"
            value={operator}
            onChange={(e) => setOperator(e.target.value as FilterOperator)}
          >
            {selectedField.type === "text" && (
              <>
                <option value="contains">contains</option>
                <option value="eq">equals</option>
                <option value="neq">not equals</option>
              </>
            )}
            {(selectedField.type === "number" || selectedField.type === "range") && (
              <>
                <option value="eq">= equals</option>
                <option value="gt">&gt; greater than</option>
                <option value="gte">≥ at least</option>
                <option value="lt">&lt; less than</option>
                <option value="lte">≤ at most</option>
              </>
            )}
          </select>
        </div>
      )}

      {/* Value */}
      {selectedField?.type === "select" && (
        <div className="flex flex-col gap-1 min-w-[160px]">
          <label className="text-[9px] font-mono uppercase text-muted-foreground">Values</label>
          <div className="flex flex-wrap gap-1">
            {selectedField.options?.map((opt) => (
              <button
                key={opt.value}
                onClick={() =>
                  setSelectedOptions((prev) =>
                    prev.includes(opt.value)
                      ? prev.filter((x) => x !== opt.value)
                      : [...prev, opt.value]
                  )
                }
                className={`px-2 py-0.5 rounded-full text-[10px] font-mono border transition-colors ${
                  selectedOptions.includes(opt.value)
                    ? "bg-accent text-white border-accent"
                    : "border-border/40 text-muted-foreground hover:border-accent/50"
                }`}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>
      )}

      {(selectedField?.type === "text" || selectedField?.type === "number") && (
        <div className="flex flex-col gap-1 min-w-[160px]">
          <label className="text-[9px] font-mono uppercase text-muted-foreground">Value</label>
          <Input
            type={selectedField.type === "number" ? "number" : "text"}
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder={selectedField.placeholder ?? "Enter value…"}
            className="h-8 text-xs font-mono"
          />
        </div>
      )}

      {selectedField?.type === "range" && (
        <div className="flex flex-col gap-1 min-w-[180px]">
          <label className="text-[9px] font-mono uppercase text-muted-foreground">
            Value: {value || selectedField.min}
          </label>
          <input
            type="range"
            min={selectedField.min ?? 0}
            max={selectedField.max ?? 100}
            step={selectedField.step ?? 1}
            value={value || String(selectedField.min ?? 0)}
            onChange={(e) => setValue(e.target.value)}
            className="accent-accent"
          />
        </div>
      )}

      {/* Actions */}
      <div className="flex items-end gap-2">
        <Button
          size="sm"
          className="h-8 text-[10px] font-mono uppercase"
          onClick={handleAdd}
          disabled={!selectedField}
        >
          Add
        </Button>
        <Button
          size="sm"
          variant="outline"
          className="h-8 text-[10px] font-mono uppercase"
          onClick={onCancel}
        >
          Cancel
        </Button>
      </div>
    </div>
  );
}

// ── Main component ────────────────────────────────────────────────────────────

export function AdvancedFilterPanel({
  fields,
  onChange,
  storageKey = "advanced_filter_saved_searches",
  placeholder = "Search…",
  className = "",
}: AdvancedFilterProps) {
  const [query, setQuery] = useState("");
  const [activeFilters, setActiveFilters] = useState<ActiveFilter[]>([]);
  const [showFilterRow, setShowFilterRow] = useState(false);
  const [savedSearches, setSavedSearches] = useState<SavedSearch[]>([]);
  const [showSaved, setShowSaved] = useState(false);
  const [saveNameInput, setSaveNameInput] = useState("");
  const savedPanelRef = useRef<HTMLDivElement>(null);

  // Load saved searches from localStorage on mount
  useEffect(() => {
    setSavedSearches(loadSavedSearches(storageKey));
  }, [storageKey]);

  // Notify parent whenever state changes
  useEffect(() => {
    onChange({ query, filters: activeFilters });
  }, [query, activeFilters, onChange]);

  // Close saved-searches dropdown on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (savedPanelRef.current && !savedPanelRef.current.contains(e.target as Node)) {
        setShowSaved(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const removeFilter = useCallback(
    (idx: number) => setActiveFilters((prev) => prev.filter((_, i) => i !== idx)),
    []
  );

  const clearAll = useCallback(() => {
    setQuery("");
    setActiveFilters([]);
  }, []);

  const handleAddFilter = useCallback((f: ActiveFilter) => {
    setActiveFilters((prev) => [...prev, f]);
    setShowFilterRow(false);
  }, []);

  const saveSearch = () => {
    if (!saveNameInput.trim()) return;
    const search: SavedSearch = {
      id: crypto.randomUUID(),
      name: saveNameInput.trim(),
      filters: activeFilters,
      query,
      savedAt: new Date().toISOString(),
    };
    const updated = [search, ...savedSearches].slice(0, 20);
    setSavedSearches(updated);
    persistSavedSearches(storageKey, updated);
    setSaveNameInput("");
  };

  const applySavedSearch = (s: SavedSearch) => {
    setQuery(s.query);
    setActiveFilters(s.filters);
    setShowSaved(false);
  };

  const deleteSavedSearch = (id: string) => {
    const updated = savedSearches.filter((s) => s.id !== id);
    setSavedSearches(updated);
    persistSavedSearches(storageKey, updated);
  };

  const hasActive = query.length > 0 || activeFilters.length > 0;

  return (
    <div className={`space-y-3 ${className}`}>
      {/* Search bar row */}
      <div className="flex items-center gap-2 flex-wrap">
        {/* Text search */}
        <div className="relative flex-1 min-w-[200px]">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground pointer-events-none" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={placeholder}
            className="pl-8 h-9 text-xs font-mono"
            aria-label="Search"
          />
          {query && (
            <button
              onClick={() => setQuery("")}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
              aria-label="Clear search"
            >
              <X className="w-3 h-3" />
            </button>
          )}
        </div>

        {/* Add filter button */}
        <button
          onClick={() => setShowFilterRow((v) => !v)}
          className="flex items-center gap-1.5 px-3 py-2 rounded-lg border border-border/40 text-[10px] font-mono uppercase tracking-widest hover:border-accent/50 transition-colors"
          aria-expanded={showFilterRow}
          aria-label="Add filter"
        >
          <SlidersHorizontal className="w-3 h-3" />
          Filter
          {activeFilters.length > 0 && (
            <Badge className="text-[9px] h-4 px-1.5 ml-1 bg-accent text-white border-0">
              {activeFilters.length}
            </Badge>
          )}
        </button>

        {/* Saved searches */}
        <div className="relative" ref={savedPanelRef}>
          <button
            onClick={() => setShowSaved((v) => !v)}
            className="flex items-center gap-1.5 px-3 py-2 rounded-lg border border-border/40 text-[10px] font-mono uppercase tracking-widest hover:border-accent/50 transition-colors"
            aria-label="Saved searches"
          >
            {savedSearches.length > 0 ? (
              <BookmarkCheck className="w-3 h-3 text-accent" />
            ) : (
              <Bookmark className="w-3 h-3" />
            )}
            Saved
            <ChevronDown className="w-2.5 h-2.5 ml-0.5" />
          </button>

          {showSaved && (
            <div className="absolute right-0 top-full mt-1 z-50 w-72 bg-popover border border-border/40 rounded-xl shadow-xl p-3 space-y-2">
              {/* Save current */}
              {hasActive && (
                <div className="flex gap-2">
                  <Input
                    value={saveNameInput}
                    onChange={(e) => setSaveNameInput(e.target.value)}
                    placeholder="Name this search…"
                    className="h-7 text-[10px] font-mono"
                    onKeyDown={(e) => e.key === "Enter" && saveSearch()}
                  />
                  <Button
                    size="sm"
                    className="h-7 text-[10px] px-2"
                    onClick={saveSearch}
                    disabled={!saveNameInput.trim()}
                  >
                    Save
                  </Button>
                </div>
              )}

              {savedSearches.length === 0 && (
                <p className="text-[10px] text-muted-foreground font-mono px-1">
                  No saved searches yet.
                </p>
              )}

              <div className="space-y-1 max-h-48 overflow-y-auto">
                {savedSearches.map((s) => (
                  <div
                    key={s.id}
                    className="flex items-center justify-between px-2 py-1.5 rounded-lg hover:bg-white/5 cursor-pointer group"
                    onClick={() => applySavedSearch(s)}
                    role="button"
                    tabIndex={0}
                    onKeyDown={(e) => e.key === "Enter" && applySavedSearch(s)}
                  >
                    <div className="min-w-0">
                      <div className="text-[10px] font-mono font-bold truncate">{s.name}</div>
                      <div className="text-[9px] text-muted-foreground">
                        {s.filters.length} filter{s.filters.length !== 1 ? "s" : ""}
                        {s.query ? ` · "${s.query}"` : ""}
                      </div>
                    </div>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteSavedSearch(s.id);
                      }}
                      className="opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-red-400 ml-2"
                      aria-label={`Delete saved search ${s.name}`}
                    >
                      <X className="w-3 h-3" />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Clear all */}
        {hasActive && (
          <button
            onClick={clearAll}
            className="flex items-center gap-1 text-[10px] font-mono text-muted-foreground hover:text-red-400 transition-colors"
            aria-label="Clear all filters"
          >
            <X className="w-3 h-3" />
            Clear all
          </button>
        )}
      </div>

      {/* Filter builder row */}
      {showFilterRow && (
        <FilterRow
          fields={fields}
          onAdd={handleAddFilter}
          onCancel={() => setShowFilterRow(false)}
        />
      )}

      {/* Active filter tags */}
      {activeFilters.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {activeFilters.map((f, i) => (
            <FilterTag key={i} filter={f} onRemove={() => removeFilter(i)} />
          ))}
        </div>
      )}
    </div>
  );
}

// ── Utility: apply filters to a list of objects ───────────────────────────────

export function applyFilters<T extends Record<string, unknown>>(
  items: T[],
  state: AdvancedFilterState
): T[] {
  const { query, filters } = state;

  return items.filter((item) => {
    // Global text search across all string values
    if (query) {
      const q = query.toLowerCase();
      const matches = Object.values(item).some(
        (v) => typeof v === "string" && v.toLowerCase().includes(q)
      );
      if (!matches) return false;
    }

    // Individual filters
    for (const filter of filters) {
      const raw = item[filter.field];
      const val = filter.value;

      if (filter.operator === "in" && Array.isArray(val)) {
        if (!val.includes(String(raw))) return false;
      } else if (filter.operator === "contains") {
        if (!String(raw ?? "").toLowerCase().includes(String(val).toLowerCase())) return false;
      } else if (filter.operator === "eq") {
        if (String(raw) !== String(val)) return false;
      } else if (filter.operator === "neq") {
        if (String(raw) === String(val)) return false;
      } else {
        const numRaw = Number(raw);
        const numVal = Number(val);
        if (isNaN(numRaw) || isNaN(numVal)) continue;
        if (filter.operator === "gt" && !(numRaw > numVal)) return false;
        if (filter.operator === "gte" && !(numRaw >= numVal)) return false;
        if (filter.operator === "lt" && !(numRaw < numVal)) return false;
        if (filter.operator === "lte" && !(numRaw <= numVal)) return false;
      }
    }

    return true;
  });
}
