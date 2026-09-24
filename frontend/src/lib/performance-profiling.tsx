"use client";

import React, { Profiler, ProfilerOnRenderCallback, ReactNode } from "react";

/**
 * Development-only performance profiling helpers.
 *
 * These wrap React's built-in `Profiler` API and the browser's User Timing
 * API (`performance.mark` / `performance.measure`) so components can be
 * instrumented without pulling in a separate profiling library. All of it is
 * a no-op in production builds.
 */

const isProfilingEnabled =
  process.env.NODE_ENV === "development" && typeof performance !== "undefined";

const defaultOnRender: ProfilerOnRenderCallback = (
  id,
  phase,
  actualDuration,
  baseDuration,
  startTime,
  commitTime,
) => {
  // eslint-disable-next-line no-console
  console.debug(
    `[profiler] ${id} (${phase}) actual=${actualDuration.toFixed(2)}ms base=${baseDuration.toFixed(2)}ms`,
    { startTime, commitTime },
  );
};

/**
 * Wraps `children` in a React `Profiler` that logs render timings via
 * `console.debug` in development. Renders `children` unwrapped in production
 * so the Profiler never adds overhead outside of local development.
 */
export function DevProfiler({
  id,
  children,
  onRender = defaultOnRender,
}: {
  id: string;
  children: ReactNode;
  onRender?: ProfilerOnRenderCallback;
}) {
  if (!isProfilingEnabled) return <>{children}</>;

  return (
    <Profiler id={id} onRender={onRender}>
      {children}
    </Profiler>
  );
}

/**
 * Starts a User Timing API mark named `${label}-start`. Call the returned
 * function to record the matching `-end` mark and a measure spanning the two,
 * visible in the browser's Performance panel and via
 * `performance.getEntriesByName(label)`. No-op in production.
 */
export function markStart(label: string): () => void {
  if (!isProfilingEnabled) return () => {};

  const startMark = `${label}-start`;
  performance.mark(startMark);

  return () => {
    const endMark = `${label}-end`;
    performance.mark(endMark);
    try {
      performance.measure(label, startMark, endMark);
    } catch {
      // Ignore if the start mark was cleared (e.g. by a page navigation).
    }
  };
}
