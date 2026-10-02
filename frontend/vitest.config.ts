import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/__tests__/setup.ts'],
    // src/config.ts and src/lib/monitoring.ts throw at import time without it.
    env: {
      NEXT_PUBLIC_API_URL: 'http://localhost:8080/api',
    },
    include: [
      'src/**/*.{test,spec}.{ts,tsx}',
    ],
    // Playwright specs live in src/__tests__/e2e and run via playwright.config.ts.
    exclude: ['node_modules/**', 'src/__tests__/e2e/**'],
    // next-intl's ESM build imports 'next/navigation' without an extension,
    // which Node's resolver rejects unless Vite transforms the package.
    server: {
      deps: {
        inline: ['next-intl'],
      },
    },
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov', 'json-summary'],
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        'src/**/*.d.ts',
        'src/**/node_modules/**',
      ],
      thresholds: {
        lines: 80,
        functions: 80,
        branches: 80,
        statements: 80,
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
