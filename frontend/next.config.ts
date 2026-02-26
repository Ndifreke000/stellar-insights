import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const withNextIntl = createNextIntlPlugin("./src/i18n/request.ts");

const nextConfig: NextConfig = {
  images: {
    // Modern image formats for better compression
    formats: ['image/webp', 'image/avif'],

    // Responsive image sizes for different devices
    deviceSizes: [640, 750, 828, 1080, 1200, 1920, 2048, 3840],
    imageSizes: [16, 32, 48, 64, 96, 128, 256, 384],

    // Cache optimized images for 30 days (2592000s)
    // Static assets like anchor logos change infrequently
    minimumCacheTTL: 2592000,

    // Quality options available to consumers
    qualities: [50, 75, 85, 95],

    // Allow external image domains (add your CDN domains here)
    remotePatterns: [
      {
        protocol: 'https',
        hostname: '**.stellar.org',
      },
      // Add more patterns as needed for anchor logos and external images
      // {
      //   protocol: 'https',
      //   hostname: 'cdn.stellar-insights.com',
      // },
    ],
  },
};

export default withNextIntl(nextConfig);
