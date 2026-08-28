/**
 * Dynamic imports for heavy chart and analytics components.
 *
 * Using Next.js `dynamic()` defers loading of recharts, framer-motion, and
 * other large dependencies until the component is actually rendered, reducing
 * the initial JS bundle size and improving Time-to-Interactive.
 *
 * Usage:
 *   import { DynamicReliabilityTrend } from "@/components/dynamic-imports";
 *
 * To analyse bundle sizes:
 *   ANALYZE=true npm run build
 */
import dynamic from "next/dynamic";

const loadingPlaceholder = () => null;

export const DynamicReliabilityTrend = dynamic(
  () => import("./charts/ReliabilityTrend").then((m) => ({ default: m.ReliabilityTrend })),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicSettlementLatencyChart = dynamic(
  () => import("./charts/SettlementLatencyChart").then((m) => ({ default: m.SettlementLatencyChart })),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicLiquidityChart = dynamic(
  () => import("./charts/LiquidityChart").then((m) => ({ default: m.LiquidityChart })),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicLiquidityHeatmap = dynamic(
  () => import("./charts/LiquidityHeatmap").then((m) => ({ default: m.LiquidityHeatmap })),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicTVLChart = dynamic(
  () => import("./charts/TVLChart").then((m) => ({ default: m.TVLChart })),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicPoolPerformanceChart = dynamic(
  () => import("./charts/PoolPerformanceChart").then((m) => ({ default: m.PoolPerformanceChart })),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicTrustlineGrowthChart = dynamic(
  () => import("./charts/TrustlineGrowthChart").then((m) => ({ default: m.TrustlineGrowthChart })),
  { ssr: false, loading: loadingPlaceholder }
);

// CorridorCompareCharts.tsx has three named exports (no single default), so
// each gets its own dynamic wrapper rather than guessing which one is "the"
// component.
export const DynamicSuccessRateCompareChart = dynamic(
  () => import("./corridors/CorridorCompareCharts").then((m) => ({ default: m.SuccessRateCompareChart })),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicVolumeCompareChart = dynamic(
  () => import("./corridors/CorridorCompareCharts").then((m) => ({ default: m.VolumeCompareChart })),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicSlippageCompareChart = dynamic(
  () => import("./corridors/CorridorCompareCharts").then((m) => ({ default: m.SlippageCompareChart })),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicNetworkGraph = dynamic(
  () => import("./charts/NetworkGraph"),
  { ssr: false, loading: loadingPlaceholder }
);
