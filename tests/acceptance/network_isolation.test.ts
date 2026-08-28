/**
 * Acceptance test: testnet ↔ mainnet data isolation
 *
 * Verifies that a transaction submitted on testnet never leaks into the
 * mainnet dashboard view and reappears correctly when switching back.
 *
 * Requires a running dev server (BASE_URL env var, default http://localhost:3000).
 */
import { test, expect, Page } from "@playwright/test";

const BASE_URL = process.env.BASE_URL ?? "http://localhost:3000";

// Selectors — adjust if the markup changes.
const NETWORK_SWITCHER = '[data-testid="network-switcher"]';
const NETWORK_OPTION = (name: "testnet" | "mainnet") =>
  `[data-testid="network-option-${name}"]`;
const CONFIRM_SWITCH_BTN = '[data-testid="confirm-network-switch"]';
const TRANSACTION_LIST = '[data-testid="transaction-list"]';
const TRANSACTION_ITEM = '[data-testid="transaction-item"]';

// Unique memo used to identify the synthetic testnet transaction.
const TEST_MEMO = `isolation-test-${Date.now()}`;

async function switchNetwork(page: Page, target: "testnet" | "mainnet") {
  await page.click(NETWORK_SWITCHER);
  await page.click(NETWORK_OPTION(target));
  // The NetworkSwitcher shows a confirmation warning before switching.
  await page.click(CONFIRM_SWITCH_BTN);
  await page.waitForResponse((res) =>
    res.url().includes("/api/network/switch") && res.status() === 200
  );
}

async function injectTestnetTransaction(page: Page): Promise<void> {
  // POST a synthetic payment to the backend test-seeding endpoint so the
  // dashboard can display it without requiring a real Stellar wallet.
  const response = await page.request.post(`${BASE_URL}/api/test/seed-transaction`, {
    data: {
      network: "testnet",
      memo: TEST_MEMO,
      source_account: "TESTNET_SEED_ACCOUNT",
      destination_account: "TESTNET_DEST_ACCOUNT",
      amount: "1.0000000",
      asset_code: "XLM",
    },
  });
  expect(response.ok()).toBeTruthy();
}

async function transactionVisible(page: Page): Promise<boolean> {
  const items = page.locator(TRANSACTION_ITEM);
  const count = await items.count();
  for (let i = 0; i < count; i++) {
    const text = await items.nth(i).textContent();
    if (text?.includes(TEST_MEMO)) return true;
  }
  return false;
}

test.describe("Network switchover — testnet → mainnet data isolation", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
  });

  test("testnet transaction is visible on testnet dashboard", async ({ page }) => {
    // Ensure we start on testnet.
    await switchNetwork(page, "testnet");

    await injectTestnetTransaction(page);

    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForSelector(TRANSACTION_LIST);

    expect(await transactionVisible(page)).toBe(true);
  });

  test("testnet transaction is NOT visible after switching to mainnet", async ({
    page,
  }) => {
    // Start on testnet and seed the transaction.
    await switchNetwork(page, "testnet");
    await injectTestnetTransaction(page);

    // Switch to mainnet.
    await switchNetwork(page, "mainnet");

    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForSelector(TRANSACTION_LIST);

    expect(await transactionVisible(page)).toBe(false);
  });

  test("testnet transaction reappears after switching back to testnet", async ({
    page,
  }) => {
    // Seed the transaction on testnet.
    await switchNetwork(page, "testnet");
    await injectTestnetTransaction(page);

    // Switch to mainnet, verify it's gone.
    await switchNetwork(page, "mainnet");
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForSelector(TRANSACTION_LIST);
    expect(await transactionVisible(page)).toBe(false);

    // Switch back to testnet.
    await switchNetwork(page, "testnet");
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForSelector(TRANSACTION_LIST);

    expect(await transactionVisible(page)).toBe(true);
  });
});
