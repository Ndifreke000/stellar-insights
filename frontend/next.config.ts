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

/**
 * Security headers applied to every route via next.config.ts.
 * The middleware (src/middleware.ts) also sets these at runtime so they are
 * present on both static and dynamic responses.
 *
 * `upgrade-insecure-requests` is omitted here because next.config.ts headers
 * run in all environments; the middleware applies it in production only.
 */
const securityHeaders = [
  {
    key: "Content-Security-Policy",
    value: [
      "default-src 'self'",
      "script-src 'self' 'unsafe-inline'",
      "style-src 'self' 'unsafe-inline'",
      "img-src 'self' data: blob: https://*.stellar.org",
      "font-src 'self'",
      "connect-src 'self' wss: https: https://*.sentry.io",
      "frame-src 'none'",
      "frame-ancestors 'none'",
      "object-src 'none'",
      "base-uri 'self'",
      "form-action 'self'",
    ].join("; "),
  },
  { key: "X-Frame-Options", value: "DENY" },
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  {
    key: "Permissions-Policy",
    value: "camera=(), microphone=(), geolocation=()",
  },
];

const nextConfig: NextConfig = {
  output: 'standalone',
  compress: true,
  async headers() {
    return [
      {
        source: "/(.*)",
        headers: securityHeaders,
      },
    ];
  },
  experimental: {
    optimizePackageImports: [
      "lucide-react",
      "recharts",
      "framer-motion",
      "@stellar/stellar-sdk",
      "d3-force-3d",
      "react-force-graph-2d",
    ],
  },
  turbopack: {
    root: '../',
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
  webpack: (config, { isServer }) => {
    if (!isServer) {
      // Performance budgets — raw sizes (pre-gzip).
      // 500 KB raw ≈ ~200 KB gzipped, matching the documented budget.
      // In CI (process.env.CI=true) violations are errors; locally they are warnings.
      config.performance = {
        hints: process.env.CI ? 'error' : 'warning',
        maxAssetSize: 500 * 1024,       // 500 KB per asset
        maxEntrypointSize: 500 * 1024,  // 500 KB per entrypoint
      };

      config.optimization = {
        ...config.optimization,
        splitChunks: {
          chunks: 'all',
          cacheGroups: {
            default: false,
            vendors: false,
            vendor: {
              filename: 'chunks/vendor.js',
              test: /node_modules/,
              priority: 10,
              reuseExistingChunk: true,
              name: 'vendor',
            },
            charts: {
              filename: 'chunks/charts.js',
              test: /[\\/]node_modules[\\/](recharts|d3-force-3d|react-force-graph-2d)[\\/]/,
              priority: 20,
              reuseExistingChunk: true,
              name: 'charts',
            },
            animation: {
              filename: 'chunks/animation.js',
              test: /[\\/]node_modules[\\/](framer-motion)[\\/]/,
              priority: 20,
              reuseExistingChunk: true,
              name: 'animation',
            },
            common: {
              filename: 'chunks/common.js',
              minChunks: 2,
              priority: 5,
              reuseExistingChunk: true,
              name: 'common',
            },
          },
        },
      };
    }
    return config;
  },
};

export default analyzer(withNextIntl(withPWA({
  dest: "public",
  cacheOnFrontEndNav: true,
  aggressiveFrontEndNavCaching: true,
  reloadOnOnline: true,
  disable: process.env.NODE_ENV === "development",
  fallbacks: {
    document: "/offline",
  },
  workboxOptions: {
    disableDevLogs: true,
  },
})(nextConfig)));
