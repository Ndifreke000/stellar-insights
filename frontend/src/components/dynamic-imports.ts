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
  () => import("./charts/ReliabilityTrend"),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicSettlementLatencyChart = dynamic(
  () => import("./charts/SettlementLatencyChart"),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicLiquidityChart = dynamic(
  () => import("./charts/LiquidityChart"),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicLiquidityHeatmap = dynamic(
  () => import("./charts/LiquidityHeatmap"),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicTVLChart = dynamic(
  () => import("./charts/TVLChart"),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicPoolPerformanceChart = dynamic(
  () => import("./charts/PoolPerformanceChart"),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicTrustlineGrowthChart = dynamic(
  () => import("./charts/TrustlineGrowthChart"),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicCorridorCompareCharts = dynamic(
  () => import("./corridors/CorridorCompareCharts"),
  { ssr: false, loading: loadingPlaceholder }
);

export const DynamicNetworkGraph = dynamic(
  () => import("./charts/NetworkGraph"),
  { ssr: false, loading: loadingPlaceholder }
);
