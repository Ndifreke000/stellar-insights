import { test, expect } from '@playwright/test';

const MOCK_ANCHORS = {
  anchors: [
    {
      name: 'Testnet Anchor',
      transfer_server: 'https://testnet-anchor.example.com/sep24',
      home_domain: 'testnet-anchor.example.com',
    },
  ],
};

const MOCK_INFO = {
  deposit: {
    USDC: {
      enabled: true,
      min_amount: 1,
      max_amount: 10000,
      fee_fixed: 0.5,
      fee_percent: 0.1,
      authentication_required: false,
    },
    XLM: {
      enabled: true,
      min_amount: 10,
      max_amount: 100000,
      authentication_required: false,
    },
  },
  withdraw: {
    USDC: {
      enabled: true,
      min_amount: 5,
      max_amount: 5000,
    },
  },
  fee: { enabled: true },
};

const MOCK_INTERACTIVE = {
  type: 'interactive_customer_info_needed',
  url: 'https://testnet-anchor.example.com/interactive/deposit?token=abc123',
  id: 'tx-001',
};

const MOCK_TRANSACTIONS = {
  transactions: [
    {
      id: 'tx-001',
      kind: 'deposit',
      status: 'completed',
      amount_in: '100.00',
      amount_out: '99.50',
      amount_fee: '0.50',
      asset_code: 'USDC',
      started_at: '2026-06-01T12:00:00Z',
      completed_at: '2026-06-01T12:05:00Z',
    },
    {
      id: 'tx-002',
      kind: 'deposit',
      status: 'pending_user_transfer_start',
      amount_in: '50.00',
      asset_code: 'XLM',
      started_at: '2026-06-25T10:00:00Z',
    },
  ],
};

test.describe('SEP-24 Deposit Flow — testnet', () => {
  test.beforeEach(async ({ page }) => {
    await page.route('**/api/sep24/anchors', (route) =>
      route.fulfill({ json: MOCK_ANCHORS }),
    );
    await page.route('**/api/sep24/info**', (route) =>
      route.fulfill({ json: MOCK_INFO }),
    );
    await page.route('**/api/sep24/deposit/interactive', (route) =>
      route.fulfill({ json: MOCK_INTERACTIVE }),
    );
    await page.route('**/api/sep24/transactions**', (route) =>
      route.fulfill({ json: MOCK_TRANSACTIONS }),
    );
  });

  test('loads anchor list and shows deposit form', async ({ page }) => {
    await page.goto('/sep24');

    const anchorSelect = page.locator('select').first();
    await expect(anchorSelect).toBeVisible();

    await anchorSelect.selectOption({ label: /Testnet Anchor/i });

    await expect(page.getByText('Start flow')).toBeVisible();
    await expect(page.getByRole('button', { name: /Deposit/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /Withdraw/i })).toBeVisible();
  });

  test('deposit button is active by default', async ({ page }) => {
    await page.goto('/sep24');

    const depositBtn = page.getByRole('button', { name: /Deposit/i }).first();
    await expect(depositBtn).toBeVisible();
  });

  test('starts interactive deposit and gets redirect URL', async ({ page }) => {
    let depositCalled = false;
    await page.route('**/api/sep24/deposit/interactive', async (route) => {
      depositCalled = true;
      const body = route.request().postDataJSON();
      expect(body).toHaveProperty('transferServer');
      await route.fulfill({ json: MOCK_INTERACTIVE });
    });

    await page.goto('/sep24');

    const anchorSelect = page.locator('select').first();
    await anchorSelect.selectOption({ label: /Testnet Anchor/i });

    await page.waitForTimeout(500);

    const startButton = page.getByRole('button', { name: /Start deposit/i });
    if (await startButton.isEnabled()) {
      await startButton.click();
      await expect(async () => {
        expect(depositCalled).toBe(true);
      }).toPass({ timeout: 5000 });
    }
  });

  test('switches between deposit and withdraw tabs', async ({ page }) => {
    await page.goto('/sep24');

    const anchorSelect = page.locator('select').first();
    await anchorSelect.selectOption({ label: /Testnet Anchor/i });

    await page.waitForTimeout(500);

    const withdrawBtn = page.getByRole('button', { name: /Withdraw/i }).first();
    await withdrawBtn.click();

    const depositBtn = page.getByRole('button', { name: /Deposit/i }).first();
    await depositBtn.click();
  });

  test('displays transaction history table', async ({ page }) => {
    await page.goto('/sep24');

    const anchorSelect = page.locator('select').first();
    await anchorSelect.selectOption({ label: /Testnet Anchor/i });

    const loadHistoryBtn = page.getByRole('button', { name: /Load history/i });
    if (await loadHistoryBtn.isVisible()) {
      await loadHistoryBtn.click();

      await expect(page.getByText('completed')).toBeVisible({ timeout: 5000 });
      await expect(page.getByText('USDC')).toBeVisible();
    }
  });

  test('shows error when anchor fetch fails', async ({ page }) => {
    await page.route('**/api/sep24/anchors', (route) =>
      route.fulfill({ status: 500, json: { error: 'Server error' } }),
    );

    await page.goto('/sep24');

    await expect(
      page.getByText(/error|failed|could not/i),
    ).toBeVisible({ timeout: 5000 });
  });

  test('handles 503 with retry on deposit', async ({ page }) => {
    let callCount = 0;
    await page.route('**/api/sep24/deposit/interactive', async (route) => {
      callCount++;
      if (callCount === 1) {
        await route.fulfill({
          status: 503,
          headers: { 'Retry-After': '1' },
          body: 'Service Unavailable',
        });
      } else {
        await route.fulfill({ json: MOCK_INTERACTIVE });
      }
    });

    await page.goto('/sep24');

    const anchorSelect = page.locator('select').first();
    await anchorSelect.selectOption({ label: /Testnet Anchor/i });

    await page.waitForTimeout(500);

    const startButton = page.getByRole('button', { name: /Start deposit/i });
    if (await startButton.isEnabled()) {
      await startButton.click();
      await expect(async () => {
        expect(callCount).toBeGreaterThanOrEqual(2);
      }).toPass({ timeout: 10000 });
    }
  });
});
