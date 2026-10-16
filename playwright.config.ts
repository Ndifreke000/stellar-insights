import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  timeout: 30_000,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? "github" : "list",
  projects: [
    {
      // Frontend UI acceptance tests — needs a browser and a running dev server.
      name: "chromium",
      testDir: "./tests/acceptance",
      use: {
        ...devices["Desktop Chrome"],
        baseURL: process.env.BASE_URL ?? "http://localhost:3000",
        trace: "on-first-retry",
      },
    },
    {
      // Backend-only API flows (SEP-10, API keys, preflight, ...) — no
      // browser needed, just Playwright's `request` fixture against a
      // running backend. See tests/e2e/README.md. Run with `npm run test:e2e`.
      name: "backend-e2e",
      testDir: "./tests/e2e",
      testMatch: /.*\.test\.ts/,
      use: {
        baseURL: process.env.API_BASE_URL ?? "http://localhost:8080",
      },
    },
  ],
});
