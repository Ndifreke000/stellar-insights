'use client';

import { useEffect, useRef, useCallback } from 'react';
import { performanceProfiler, type RenderProfile } from '@/lib/performance';
import { monitoring } from '@/lib/monitoring';

interface UsePerformanceProfileOptions {
  /** Report metrics to the monitoring singleton. Defaults to true. */
  report?: boolean;
}

interface UsePerformanceProfileReturn {
  /** Call at the top of a render to start timing. */
  markRenderStart: () => void;
  /** Call at the bottom of a render (or in useEffect) to end timing. */
  markRenderEnd: () => RenderProfile | null;
  /** Snapshot current memory usage and optionally report it. */
  snapshotMemory: () => void;
}

/**
 * Hook for profiling component render performance.
 *
 * Usage:
 *   const { markRenderStart, markRenderEnd } = usePerformanceProfile('MyComponent');
 *   markRenderStart();          // top of component body
 *   // ... render logic ...
 *   useEffect(markRenderEnd);   // after paint
 */
export function usePerformanceProfile(
  componentName: string,
  { report = true }: UsePerformanceProfileOptions = {},
): UsePerformanceProfileReturn {
  const startMarkRef = useRef<string | null>(null);

  const markRenderStart = useCallback(() => {
    startMarkRef.current = performanceProfiler.startRender(componentName);
  }, [componentName]);

  const markRenderEnd = useCallback((): RenderProfile | null => {
    if (!startMarkRef.current) return null;
    const profile = performanceProfiler.endRender(componentName, startMarkRef.current);
    startMarkRef.current = null;

    if (report) {
      monitoring.trackMetric(`render:${componentName}`, profile.renderMs, {
        slow: profile.slow,
      });
    }

    return profile;
  }, [componentName, report]);

  const snapshotMemory = useCallback(() => {
    const snap = performanceProfiler.snapshotMemory();
    if (snap && report) {
      monitoring.trackMetric('memory:usedJSHeapMB', snap.usedJSHeapMB, {
        totalJSHeapMB: snap.totalJSHeapMB,
      });
    }
  }, [report]);

  // Report navigation timing once on mount
  useEffect(() => {
    if (!report) return;
    const timing = performanceProfiler.getNavigationTiming();
    if (timing) {
      Object.entries(timing).forEach(([key, value]) => {
        monitoring.trackMetric(`nav:${key}`, value);
      });
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return { markRenderStart, markRenderEnd, snapshotMemory };
}
