import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";
import withBundleAnalyzer from "@next/bundle-analyzer";
import withPWA from "@ducanh2912/next-pwa";

const withNextIntl = createNextIntlPlugin("./src/i18n/request.ts");

const analyzer = withBundleAnalyzer({
  enabled: process.env.ANALYZE === 'true',
});

/**
 * Code splitting strategy (#1136)
 *
 * Heavy chart/analytics components (recharts, framer-motion, NetworkGraph, etc.)
 * are loaded on-demand via Next.js dynamic() in:
 *   src/components/dynamic-imports.ts
 *
 * This keeps them out of the initial JS bundle, reducing Time-to-Interactive.
 * `optimizePackageImports` below enables tree-shaking for the same libraries
 * at the module level as a complementary optimisation.
 *
 * To analyse bundle sizes after a build:
 *   ANALYZE=true npm run build
 */

const nextConfig: NextConfig = {
  experimental: {
    optimizePackageImports: [
      "lucide-react",
      "recharts",
      "framer-motion",
      "@stellar/stellar-sdk",
    ],
    turbopack: {
      root: '../',
    },
  },
  images: {
    formats: ['image/webp', 'image/avif'],
    deviceSizes: [640, 750, 828, 1080, 1200, 1920, 2048, 3840],
    imageSizes: [16, 32, 48, 64, 96, 128, 256, 384],
    minimumCacheTTL: 2592000,
    qualities: [50, 75, 85, 95],
    remotePatterns: [
      {
        protocol: 'https',
        hostname: '**.stellar.org',
      },
    ],
  },
};

export default analyzer(withNextIntl(withPWA({
  dest: "public",
  cacheOnFrontEndNav: true,
  aggressiveFrontEndNavCaching: true,
  reloadOnOnline: true,
  disable: process.env.NODE_ENV === "development",
  workboxOptions: {
    disableDevLogs: true,
  },
})(nextConfig)));
