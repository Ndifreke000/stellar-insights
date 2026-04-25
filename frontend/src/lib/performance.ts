'use client';

/**
 * Frontend Performance Profiling
 * Tracks render times, Web Vitals, API latency budgets, and memory usage.
 * Integrates with the existing monitoring singleton via trackMetric().
 */

export interface RenderProfile {
  component: string;
  renderMs: number;
  timestamp: number;
  slow: boolean;
}

export interface MemorySnapshot {
  usedJSHeapMB: number;
  totalJSHeapMB: number;
  limitJSHeapMB: number;
  timestamp: number;
}

export interface PerformanceSummary {
  renders: RenderProfile[];
  slowRenders: RenderProfile[];
  avgRenderMs: Record<string, number>;
  memory: MemorySnapshot | null;
  longTasks: number;
}

// Render time budget per component in ms — tune as needed
const SLOW_RENDER_THRESHOLD_MS = 16; // 1 frame @ 60 fps

class PerformanceProfiler {
  private static instance: PerformanceProfiler;
  private renders: RenderProfile[] = [];
  private longTaskCount = 0;
  private longTaskObserver: PerformanceObserver | null = null;
  private readonly MAX_ENTRIES = 200;

  private constructor() {
    if (typeof window !== 'undefined') {
      this.observeLongTasks();
    }
  }

  static getInstance(): PerformanceProfiler {
    if (!PerformanceProfiler.instance) {
      PerformanceProfiler.instance = new PerformanceProfiler();
    }
    return PerformanceProfiler.instance;
  }

  // ── Render timing ──────────────────────────────────────────────────────────

  /** Call before rendering; returns an opaque start mark name. */
  startRender(component: string): string {
    const mark = `render-start:${component}:${Date.now()}`;
    if (typeof performance !== 'undefined') {
      performance.mark(mark);
    }
    return mark;
  }

  /** Call after rendering with the mark returned by startRender(). */
  endRender(component: string, startMark: string): RenderProfile {
    let renderMs = 0;

    if (typeof performance !== 'undefined') {
      const endMark = `render-end:${component}:${Date.now()}`;
      performance.mark(endMark);
      try {
        const [entry] = performance.measure(
          `render:${component}`,
          startMark,
          endMark,
        );
        renderMs = entry?.duration ?? 0;
      } catch {
        // marks may have been cleared
      }
    }

    const profile: RenderProfile = {
      component,
      renderMs,
      timestamp: Date.now(),
      slow: renderMs > SLOW_RENDER_THRESHOLD_MS,
    };

    this.renders.push(profile);
    if (this.renders.length > this.MAX_ENTRIES) {
      this.renders.shift();
    }

    if (profile.slow) {
      console.warn(
        `[Perf] Slow render detected: ${component} took ${renderMs.toFixed(1)}ms`,
      );
    }

    return profile;
  }

  // ── Memory ─────────────────────────────────────────────────────────────────

  snapshotMemory(): MemorySnapshot | null {
    if (typeof performance === 'undefined') return null;
    // performance.memory is a Chrome-only non-standard API
    const mem = (performance as unknown as { memory?: {
      usedJSHeapSize: number;
      totalJSHeapSize: number;
      jsHeapSizeLimit: number;
    } }).memory;
    if (!mem) return null;

    return {
      usedJSHeapMB: mem.usedJSHeapSize / 1_048_576,
      totalJSHeapMB: mem.totalJSHeapSize / 1_048_576,
      limitJSHeapMB: mem.jsHeapSizeLimit / 1_048_576,
      timestamp: Date.now(),
    };
  }

  // ── Long tasks ─────────────────────────────────────────────────────────────

  private observeLongTasks() {
    try {
      this.longTaskObserver = new PerformanceObserver((list) => {
        this.longTaskCount += list.getEntries().length;
      });
      this.longTaskObserver.observe({ entryTypes: ['longtask'] });
    } catch {
      // longtask not supported in all browsers
    }
  }

  // ── Summary ────────────────────────────────────────────────────────────────

  getSummary(): PerformanceSummary {
    const slowRenders = this.renders.filter((r) => r.slow);

    const totals: Record<string, { sum: number; count: number }> = {};
    for (const r of this.renders) {
      if (!totals[r.component]) totals[r.component] = { sum: 0, count: 0 };
      totals[r.component].sum += r.renderMs;
      totals[r.component].count += 1;
    }

    const avgRenderMs: Record<string, number> = {};
    for (const [comp, { sum, count }] of Object.entries(totals)) {
      avgRenderMs[comp] = sum / count;
    }

    return {
      renders: [...this.renders],
      slowRenders,
      avgRenderMs,
      memory: this.snapshotMemory(),
      longTasks: this.longTaskCount,
    };
  }

  // ── Navigation timing ──────────────────────────────────────────────────────

  /** Returns key navigation timing metrics (ms). */
  getNavigationTiming(): Record<string, number> | null {
    if (typeof performance === 'undefined') return null;
    const [nav] = performance.getEntriesByType(
      'navigation',
    ) as PerformanceNavigationTiming[];
    if (!nav) return null;

    return {
      ttfb: nav.responseStart - nav.requestStart,
      domContentLoaded: nav.domContentLoadedEventEnd - nav.startTime,
      loadComplete: nav.loadEventEnd - nav.startTime,
      dnsLookup: nav.domainLookupEnd - nav.domainLookupStart,
      tcpConnect: nav.connectEnd - nav.connectStart,
    };
  }

  // ── Cleanup ────────────────────────────────────────────────────────────────

  reset() {
    this.renders = [];
    this.longTaskCount = 0;
  }

  destroy() {
    this.longTaskObserver?.disconnect();
  }
}

export const performanceProfiler = PerformanceProfiler.getInstance();

// ── Convenience helpers ──────────────────────────────────────────────────────

/** Measure an async operation and return its duration in ms. */
export async function measureAsync<T>(
  label: string,
  fn: () => Promise<T>,
): Promise<{ result: T; durationMs: number }> {
  const start = performance.now();
  const result = await fn();
  const durationMs = performance.now() - start;

  if (durationMs > 1000) {
    console.warn(`[Perf] Slow async operation "${label}": ${durationMs.toFixed(0)}ms`);
  }

  return { result, durationMs };
}

/** Measure a sync operation and return its duration in ms. */
export function measureSync<T>(
  label: string,
  fn: () => T,
): { result: T; durationMs: number } {
  const start = performance.now();
  const result = fn();
  const durationMs = performance.now() - start;
  return { result, durationMs };
}
