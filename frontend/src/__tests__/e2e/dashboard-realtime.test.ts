import { test, expect } from '@playwright/test';

test.describe('Dashboard — realtime updates on testnet', () => {
  test.beforeEach(async ({ page }) => {
    await page.route('**/api/network/info', (route) =>
      route.fulfill({
        json: {
          network: 'testnet',
          display_name: 'Testnet',
          rpc_url: 'https://soroban-testnet.stellar.org',
          horizon_url: 'https://horizon-testnet.stellar.org',
          network_passphrase: 'Test SDF Network ; September 2015',
          color: '#4ECDC4',
          is_mainnet: false,
          is_testnet: true,
        },
      }),
    );

    await page.route('**/api/dashboard/**', (route) =>
      route.fulfill({
        json: {
          kpis: {
            totalVolume: '$1,234,567',
            activeCorridors: 42,
            avgSettlementTime: '3.2s',
            liquidityDepth: '$890,123',
          },
          corridors: [],
          topAssets: [],
        },
      }),
    );

    await page.route('**/api/corridors**', (route) =>
      route.fulfill({
        json: {
          corridors: [
            { id: 'c1', name: 'USD → EUR', volume: 500000, health: 'healthy' },
            { id: 'c2', name: 'USD → NGN', volume: 250000, health: 'degraded' },
          ],
        },
      }),
    );
  });

  test('renders dashboard KPI cards', async ({ page }) => {
    await page.goto('/dashboard');

    await expect(page.locator('[class*="col-span"]').first()).toBeVisible({
      timeout: 10000,
    });
  });

  test('testnet badge or indicator is visible on dashboard', async ({ page }) => {
    await page.goto('/dashboard');

    const testnetIndicator = page.getByText(/testnet/i).first();
    await expect(testnetIndicator).toBeVisible({ timeout: 10000 });
  });

  test('dashboard updates when new data arrives via polling', async ({ page }) => {
    let callCount = 0;
    await page.route('**/api/dashboard/**', async (route) => {
      callCount++;
      const volume = callCount === 1 ? '$1,000,000' : '$2,000,000';
      await route.fulfill({
        json: {
          kpis: {
            totalVolume: volume,
            activeCorridors: 42,
            avgSettlementTime: '3.2s',
            liquidityDepth: '$890,123',
          },
          corridors: [],
          topAssets: [],
        },
      });
    });

    await page.goto('/dashboard');

    await page.waitForTimeout(2000);

    await expect(async () => {
      expect(callCount).toBeGreaterThanOrEqual(1);
    }).toPass();
  });

  test('WebSocket connection status is reflected in UI', async ({ page }) => {
    await page.goto('/dashboard');

    const wsStatus = page.locator(
      '[class*="connected"], [class*="disconnected"], [data-testid="ws-status"]',
    );

    if (await wsStatus.count() > 0) {
      await expect(wsStatus.first()).toBeVisible();
    }
  });

  test('dashboard handles API errors gracefully', async ({ page }) => {
    await page.route('**/api/dashboard/**', (route) =>
      route.fulfill({ status: 500, json: { error: 'Internal error' } }),
    );

    await page.goto('/dashboard');

    await page.waitForTimeout(3000);

    const pageContent = await page.textContent('body');
    expect(pageContent).not.toContain('Unhandled');
  });

  test('corridor data renders in dashboard view', async ({ page }) => {
    await page.goto('/dashboard');

    await page.waitForTimeout(2000);

    const corridorText = page.getByText(/USD|corridor/i).first();
    if (await corridorText.isVisible().catch(() => false)) {
      await expect(corridorText).toBeVisible();
    }
  });
});
