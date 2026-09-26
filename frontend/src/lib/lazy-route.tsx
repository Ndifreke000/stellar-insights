/**
 * Lazy Route Loader — wraps Next.js dynamic() for route-level code splitting.
 *
 * Heavy page components that pull in large dependencies (charts, 3D graphs,
 * PDF generators) are loaded on-demand, keeping the initial JS bundle small.
 *
 * Usage in a page:
 * ```tsx
 * import { lazyRoute } from '@/lib/lazy-route';
 * const AnalyticsDashboard = lazyRoute(() => import('@/components/AnalyticsDashboard'));
 * ```
 */
import dynamic from 'next/dynamic';
import type { ComponentType } from 'react';

interface LazyRouteOptions {
  /** Loading fallback component */
  loading?: () => JSX.Element;
  /** Show loading skeleton (default: true) */
  showSkeleton?: boolean;
  /** Enable SSR for this route (default: false for heavy components) */
  ssr?: boolean;
}

const DefaultLoading = () => (
  <div
    role="status"
    aria-label="Loading"
    style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      minHeight: '200px',
      width: '100%',
    }}
  >
    <div
      style={{
        width: '32px',
        height: '32px',
        border: '3px solid #e5e7eb',
        borderTopColor: '#3b82f6',
        borderRadius: '50%',
        animation: 'spin 0.8s linear infinite',
      }}
    />
    <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
  </div>
);

/**
 * Create a lazily-loaded route component with sensible defaults.
 * SSR is disabled by default since heavy components (charts, graphs)
 * are typically client-side only and would slow down TTFB if rendered server-side.
 */
export function lazyRoute<T extends ComponentType>(
  loader: () => Promise<{ default: T }>,
  options: LazyRouteOptions = {},
): T {
  const {
    loading,
    showSkeleton = true,
    ssr = false,
  } = options;

  return dynamic(loader, {
    ssr,
    loading: loading ?? (showSkeleton ? DefaultLoading : undefined),
  }) as T;
}

/**
 * Lazy-load a heavy chart component (recharts, d3-force, etc.)
 * These are always client-side only and show a loading skeleton.
 */
export function lazyChart<T extends ComponentType>(
  loader: () => Promise<{ default: T }>,
): T {
  return lazyRoute(loader, { ssr: false, showSkeleton: true });
}

/**
 * Lazy-load a PDF/export component (jspdf, html-to-image)
 * These are only needed when the user clicks "Export", so they
 * should never be in the initial bundle.
 */
export function lazyExport<T extends ComponentType>(
  loader: () => Promise<{ default: T }>,
): T {
  return lazyRoute(loader, { ssr: false, showSkeleton: false });
}
