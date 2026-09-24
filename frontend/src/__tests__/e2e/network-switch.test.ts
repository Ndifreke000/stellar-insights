import { test, expect } from '@playwright/test';

const TESTNET_NETWORK = {
  network: 'testnet',
  display_name: 'Testnet',
  rpc_url: 'https://soroban-testnet.stellar.org',
  horizon_url: 'https://horizon-testnet.stellar.org',
  network_passphrase: 'Test SDF Network ; September 2015',
  color: '#4ECDC4',
  is_mainnet: false,
  is_testnet: true,
};

const MAINNET_NETWORK = {
  network: 'mainnet',
  display_name: 'Mainnet',
  rpc_url: 'https://stellar.api.onfinality.io/public',
  horizon_url: 'https://horizon.stellar.org',
  network_passphrase: 'Public Global Stellar Network ; September 2015',
  color: '#2563EB',
  is_mainnet: true,
  is_testnet: false,
};

test.describe('Network Switcher — testnet flows', () => {
  test.beforeEach(async ({ page }) => {
    await page.route('**/api/network/info', (route) =>
      route.fulfill({ json: MAINNET_NETWORK }),
    );
    await page.route('**/api/network/available', (route) =>
      route.fulfill({ json: [MAINNET_NETWORK, TESTNET_NETWORK] }),
    );
    await page.route('**/api/network/switch', (route) =>
      route.fulfill({
        json: { message: 'Network switched to testnet. Server restart required.' },
      }),
    );

    await page.goto('/');
  });

  test('displays current network and available networks', async ({ page }) => {
    const switcher = page.getByRole('button', { name: /Network:.*Click to switch/i });
    await expect(switcher).toBeVisible();
    await expect(switcher).toContainText('Mainnet');

    await switcher.click();

    const listbox = page.getByRole('listbox', { name: /Available networks/i });
    await expect(listbox).toBeVisible();

    const mainnetOption = listbox.getByRole('option', { name: /Mainnet/i });
    const testnetOption = listbox.getByRole('option', { name: /Testnet/i });
    await expect(mainnetOption).toBeVisible();
    await expect(testnetOption).toBeVisible();
    await expect(mainnetOption).toHaveAttribute('aria-selected', 'true');
    await expect(testnetOption).toHaveAttribute('aria-selected', 'false');
  });

  test('shows confirmation dialog when switching to testnet', async ({ page }) => {
    const switcher = page.getByRole('button', { name: /Network:.*Click to switch/i });
    await switcher.click();

    const testnetOption = page.getByRole('option', { name: /Testnet/i });
    await testnetOption.click();

    const dialog = page.getByRole('dialog', { name: /Switch Network/i });
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText('Mainnet');
    await expect(dialog).toContainText('Testnet');
    await expect(dialog).toContainText('testnet');
  });

  test('cancelling network switch keeps current network', async ({ page }) => {
    const switcher = page.getByRole('button', { name: /Network:.*Click to switch/i });
    await switcher.click();

    await page.getByRole('option', { name: /Testnet/i }).click();

    const cancelButton = page.getByRole('button', { name: /Cancel network switch/i });
    await cancelButton.click();

    await expect(page.getByRole('dialog')).not.toBeVisible();
    await expect(switcher).toContainText('Mainnet');
  });

  test('confirming switch sends POST and updates UI', async ({ page }) => {
    const switchRequests: string[] = [];
    await page.route('**/api/network/switch', async (route) => {
      const body = route.request().postDataJSON();
      switchRequests.push(body.network);
      await route.fulfill({
        json: { message: 'Network switched to testnet.' },
      });
    });

    page.on('dialog', (dialog) => dialog.accept());

    const switcher = page.getByRole('button', { name: /Network:.*Click to switch/i });
    await switcher.click();
    await page.getByRole('option', { name: /Testnet/i }).click();
    await page.getByRole('button', { name: /Confirm switch to Testnet/i }).click();

    await expect(async () => {
      expect(switchRequests).toContain('testnet');
    }).toPass();

    await expect(switcher).toContainText('Testnet');
  });

  test('selecting the already-active network closes dropdown without dialog', async ({ page }) => {
    const switcher = page.getByRole('button', { name: /Network:.*Click to switch/i });
    await switcher.click();
    await page.getByRole('option', { name: /Mainnet/i }).click();

    await expect(page.getByRole('dialog')).not.toBeVisible();
    await expect(page.getByRole('listbox')).not.toBeVisible();
  });

  test('shows error state when network info fails', async ({ page }) => {
    await page.route('**/api/network/info', (route) =>
      route.fulfill({ status: 500, body: 'Internal Server Error' }),
    );

    await page.goto('/');

    await expect(page.getByText('Network Error')).toBeVisible();
  });
});
