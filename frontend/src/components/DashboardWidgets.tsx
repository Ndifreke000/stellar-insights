"use client";

/**
 * #2109 – Customizable Dashboard Widgets
 *
 * Provides a drag-and-drop widget grid where users can:
 *  - Toggle widget visibility (add/remove)
 *  - Reorder widgets via drag-and-drop
 *  - Layouts are persisted per-device in localStorage (not synced to account)
 */

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { GripVertical, X, Plus, LayoutGrid, RotateCcw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

// ── Widget registry types ─────────────────────────────────────────────────────

export interface WidgetDefinition {
  id: string;
  title: string;
  description: string;
  /** default: true */
  defaultVisible?: boolean;
  /** Tailwind grid col-span class, e.g. "lg:col-span-4" */
  colSpan?: string;
  minH?: string;
}

export interface WidgetLayout {
  id: string;
  visible: boolean;
  order: number;
}

// ── Context ───────────────────────────────────────────────────────────────────

interface WidgetContextValue {
  widgets: WidgetLayout[];
  toggle: (id: string) => void;
  reorder: (dragId: string, dropId: string) => void;
  reset: () => void;
}

const WidgetContext = createContext<WidgetContextValue>({
  widgets: [],
  toggle: () => {},
  reorder: () => {},
  reset: () => {},
});

export function useWidgets() {
  return useContext(WidgetContext);
}

// ── Persistence helpers ───────────────────────────────────────────────────────

function loadLayout(storageKey: string, definitions: WidgetDefinition[]): WidgetLayout[] {
  try {
    const raw = typeof window !== "undefined" ? localStorage.getItem(storageKey) : null;
    if (!raw) return defaultLayout(definitions);
    const saved = JSON.parse(raw) as WidgetLayout[];
    // Merge: add any newly defined widgets that weren't in the saved layout
    const existing = new Set(saved.map((w) => w.id));
    const merged = [...saved];
    definitions.forEach((def, i) => {
      if (!existing.has(def.id)) {
        merged.push({ id: def.id, visible: def.defaultVisible ?? true, order: saved.length + i });
      }
    });
    return merged;
  } catch {
    return defaultLayout(definitions);
  }
}

function defaultLayout(definitions: WidgetDefinition[]): WidgetLayout[] {
  return definitions.map((def, i) => ({
    id: def.id,
    visible: def.defaultVisible ?? true,
    order: i,
  }));
}

function saveLayout(storageKey: string, layout: WidgetLayout[]): void {
  try {
    localStorage.setItem(storageKey, JSON.stringify(layout));
  } catch {
    // Ignore quota errors
  }
}

// ── Provider ──────────────────────────────────────────────────────────────────

interface WidgetProviderProps {
  definitions: WidgetDefinition[];
  storageKey?: string;
  children: React.ReactNode;
}

export function WidgetProvider({
  definitions,
  storageKey = "dashboard_widget_layout",
  children,
}: WidgetProviderProps) {
  const [widgets, setWidgets] = useState<WidgetLayout[]>([]);

  // Load from localStorage on mount
  useEffect(() => {
    setWidgets(loadLayout(storageKey, definitions));
  }, [storageKey]); // eslint-disable-line react-hooks/exhaustive-deps

  // Persist whenever layout changes
  useEffect(() => {
    if (widgets.length > 0) saveLayout(storageKey, widgets);
  }, [widgets, storageKey]);

  const toggle = useCallback((id: string) => {
    setWidgets((prev) =>
      prev.map((w) => (w.id === id ? { ...w, visible: !w.visible } : w))
    );
  }, []);

  const reorder = useCallback((dragId: string, dropId: string) => {
    if (dragId === dropId) return;
    setWidgets((prev) => {
      const sorted = [...prev].sort((a, b) => a.order - b.order);
      const dragIdx = sorted.findIndex((w) => w.id === dragId);
      const dropIdx = sorted.findIndex((w) => w.id === dropId);
      if (dragIdx === -1 || dropIdx === -1) return prev;
      const [moved] = sorted.splice(dragIdx, 1);
      sorted.splice(dropIdx, 0, moved);
      return sorted.map((w, i) => ({ ...w, order: i }));
    });
  }, []);

  const reset = useCallback(() => {
    setWidgets(defaultLayout(definitions));
  }, [definitions]);

  return (
    <WidgetContext.Provider value={{ widgets, toggle, reorder, reset }}>
      {children}
    </WidgetContext.Provider>
  );
}

// ── Draggable widget wrapper ──────────────────────────────────────────────────

interface DraggableWidgetProps {
  id: string;
  colSpan?: string;
  minH?: string;
  children: React.ReactNode;
}

export function DraggableWidget({
  id,
  colSpan = "lg:col-span-4",
  minH = "min-h-[200px]",
  children,
}: DraggableWidgetProps) {
  const { reorder } = useWidgets();
  const dragRef = useRef<HTMLDivElement>(null);

  const handleDragStart = (e: React.DragEvent) => {
    e.dataTransfer.setData("widget-id", id);
    e.dataTransfer.effectAllowed = "move";
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    dragRef.current?.classList.add("ring-2", "ring-accent/50");
  };

  const handleDragLeave = () => {
    dragRef.current?.classList.remove("ring-2", "ring-accent/50");
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    dragRef.current?.classList.remove("ring-2", "ring-accent/50");
    const dragId = e.dataTransfer.getData("widget-id");
    if (dragId && dragId !== id) reorder(dragId, id);
  };

  return (
    <div
      ref={dragRef}
      className={`${colSpan} ${minH} group relative rounded-2xl transition-all duration-200`}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      {/* Drag handle */}
      <div
        draggable
        onDragStart={handleDragStart}
        className="absolute top-2 left-2 z-10 opacity-0 group-hover:opacity-100 transition-opacity cursor-grab active:cursor-grabbing p-1 rounded-md bg-white/5 border border-white/10"
        title="Drag to reorder"
        aria-label="Drag to reorder widget"
      >
        <GripVertical className="w-3 h-3 text-muted-foreground" />
      </div>
      {children}
    </div>
  );
}

// ── Widget grid ───────────────────────────────────────────────────────────────

interface WidgetGridProps {
  definitions: WidgetDefinition[];
  renderWidget: (id: string) => React.ReactNode;
  className?: string;
}

export function WidgetGrid({ definitions, renderWidget, className = "" }: WidgetGridProps) {
  const { widgets } = useWidgets();

  const sorted = [...widgets]
    .filter((w) => w.visible)
    .sort((a, b) => a.order - b.order);

  return (
    <div className={`grid grid-cols-1 lg:grid-cols-12 gap-6 ${className}`}>
      {sorted.map((w) => {
        const def = definitions.find((d) => d.id === w.id);
        return (
          <DraggableWidget
            key={w.id}
            id={w.id}
            colSpan={def?.colSpan}
            minH={def?.minH}
          >
            {renderWidget(w.id)}
          </DraggableWidget>
        );
      })}
    </div>
  );
}

// ── Widget customizer panel ───────────────────────────────────────────────────

interface WidgetCustomizerProps {
  definitions: WidgetDefinition[];
  isOpen: boolean;
  onClose: () => void;
}

export function WidgetCustomizer({ definitions, isOpen, onClose }: WidgetCustomizerProps) {
  const { widgets, toggle, reset } = useWidgets();

  if (!isOpen) return null;

  const getLayout = (id: string) => widgets.find((w) => w.id === id);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      role="dialog"
      aria-modal="true"
      aria-label="Customize dashboard widgets"
    >
      {/* Overlay */}
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm"
        onClick={onClose}
        aria-hidden="true"
      />

      {/* Panel */}
      <div className="relative z-10 w-full max-w-md bg-background border border-border/40 rounded-2xl shadow-2xl p-6 space-y-5">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <LayoutGrid className="w-4 h-4 text-accent" />
            <h2 className="text-sm font-mono font-bold uppercase tracking-widest">
              Customise Widgets
            </h2>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={reset}
              className="flex items-center gap-1 text-[10px] font-mono text-muted-foreground hover:text-foreground transition-colors"
              title="Reset to defaults"
              aria-label="Reset widget layout to defaults"
            >
              <RotateCcw className="w-3 h-3" />
              Reset
            </button>
            <button
              onClick={onClose}
              className="text-muted-foreground hover:text-foreground transition-colors"
              aria-label="Close widget customizer"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        <p className="text-[10px] text-muted-foreground font-mono">
          Toggle widgets on or off. Drag the{" "}
          <GripVertical className="inline w-3 h-3" /> handle on any widget to
          reorder it in the grid. Layout is saved on this device only.
        </p>

        <ul className="space-y-2 max-h-[60vh] overflow-y-auto pr-1">
          {definitions.map((def) => {
            const layout = getLayout(def.id);
            const visible = layout?.visible ?? (def.defaultVisible ?? true);
            return (
              <li
                key={def.id}
                className="flex items-center justify-between p-3 rounded-xl border border-border/30 hover:border-accent/30 transition-colors"
              >
                <div className="min-w-0 mr-3">
                  <div className="text-xs font-mono font-bold">{def.title}</div>
                  <div className="text-[10px] text-muted-foreground leading-snug">
                    {def.description}
                  </div>
                </div>
                <button
                  onClick={() => toggle(def.id)}
                  className={`flex items-center gap-1 shrink-0 px-3 py-1 rounded-full text-[10px] font-mono font-bold border transition-colors ${
                    visible
                      ? "bg-accent/20 border-accent/40 text-accent hover:bg-red-500/10 hover:border-red-500/40 hover:text-red-400"
                      : "bg-white/5 border-border/40 text-muted-foreground hover:bg-accent/10 hover:border-accent/30 hover:text-accent"
                  }`}
                  aria-pressed={visible}
                  aria-label={`${visible ? "Hide" : "Show"} ${def.title} widget`}
                >
                  {visible ? (
                    <>
                      <X className="w-2.5 h-2.5" />
                      Hide
                    </>
                  ) : (
                    <>
                      <Plus className="w-2.5 h-2.5" />
                      Show
                    </>
                  )}
                </button>
              </li>
            );
          })}
        </ul>

        <div className="flex justify-end">
          <Button size="sm" onClick={onClose} className="text-[10px] font-mono uppercase">
            Done
          </Button>
        </div>
      </div>
    </div>
  );
}

// ── Customise button (convenience) ───────────────────────────────────────────

interface CustomiseButtonProps {
  onClick: () => void;
  activeCount: number;
  totalCount: number;
}

export function CustomiseButton({ onClick, activeCount, totalCount }: CustomiseButtonProps) {
  return (
    <button
      onClick={onClick}
      className="flex items-center gap-2 px-4 py-2 bg-white/5 border border-border/40 rounded-xl text-[10px] font-mono uppercase tracking-widest hover:border-accent/50 transition-colors"
      aria-label="Customise dashboard widgets"
    >
      <LayoutGrid className="w-3 h-3" />
      Customise
      <Badge className="text-[9px] h-4 px-1.5 bg-accent/20 text-accent border-0">
        {activeCount}/{totalCount}
      </Badge>
    </button>
  );
}
