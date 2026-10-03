import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import UpgradePanel from '@/app/[locale]/developer/components/UpgradePanel';
import * as apiKeys from '@/lib/api-keys';
import * as payInvoice from '@/lib/pay-invoice';

vi.mock('@/lib/api-keys');
vi.mock('@/lib/pay-invoice');

const KEY: apiKeys.ApiKeyInfo = {
  id: 'key-1',
  name: 'offramp',
  key_prefix: 'si_live_abcd1234...',
  wallet_address: 'GWALLET',
  scopes: 'read',
  status: 'active',
  created_at: '2026-10-01T00:00:00Z',
  last_used_at: null,
  expires_at: null,
  revoked_at: null,
};

const PLAN: apiKeys.BillingPlan = {
  plan: 'pro',
  price: '50',
  asset_code: 'USDC',
  asset_issuer: 'GISSUER',
  destination: 'GTREASURY',
  period_days: 30,
  limit_per_minute: 1000,
  free_limit_per_minute: 200,
  anonymous_limit_per_minute: 60,
};

const INVOICE: apiKeys.Invoice = {
  id: 'inv-1',
  api_key_id: 'key-1',
  amount_usdc: '50',
  asset_code: 'USDC',
  asset_issuer: 'GISSUER',
  destination: 'GTREASURY',
  memo: 'payraider-abc123',
  limit_per_minute: 1000,
  period_days: 30,
  status: 'pending',
  transaction_hash: null,
  created_at: '2026-10-01T00:00:00Z',
  expires_at: '2026-10-02T00:00:00Z',
};

const HASH = 'a'.repeat(64);

function renderPanel() {
  return render(<UpgradePanel authToken="token" address="GWALLET" keys={[KEY]} />);
}

describe('UpgradePanel', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(apiKeys.getBillingPlan).mockResolvedValue(PLAN);
    vi.mocked(apiKeys.getSubscription).mockResolvedValue(null);
    vi.mocked(apiKeys.createInvoice).mockResolvedValue(INVOICE);
    vi.mocked(apiKeys.confirmInvoice).mockResolvedValue({
      invoice: { ...INVOICE, status: 'paid', transaction_hash: HASH },
      subscription: { api_key_id: 'key-1', plan: 'pro', limit_per_minute: 1000, paid_until: '2099-01-01T00:00:00Z' },
    });
  });

  it('describes the plan and the free limits', async () => {
    renderPanel();
    expect(await screen.findByText(/1,000 requests a minute/)).toBeInTheDocument();
    expect(screen.getByText(/50 USDC per 30 days/)).toBeInTheDocument();
  });

  it('says so when the server has no paid plan', async () => {
    vi.mocked(apiKeys.getBillingPlan).mockResolvedValue(null);
    renderPanel();
    expect(await screen.findByText(/Paid plans are not enabled/)).toBeInTheDocument();
  });

  it('shows the payment details from the invoice', async () => {
    renderPanel();
    fireEvent.click(await screen.findByRole('button', { name: /Upgrade for 50 USDC/ }));

    expect(await screen.findByText('payraider-abc123')).toBeInTheDocument();
    expect(screen.getByText('GTREASURY')).toBeInTheDocument();
    expect(apiKeys.createInvoice).toHaveBeenCalledWith('token', 'key-1');
  });

  it('confirms a payment made from another wallet by its hash', async () => {
    renderPanel();
    fireEvent.click(await screen.findByRole('button', { name: /Upgrade for 50 USDC/ }));
    fireEvent.change(await screen.findByLabelText('Transaction hash'), { target: { value: HASH } });
    fireEvent.click(screen.getByRole('button', { name: /Confirm payment/ }));

    expect(await screen.findByText(/Payment confirmed on the ledger/)).toBeInTheDocument();
    expect(apiKeys.confirmInvoice).toHaveBeenCalledWith('token', 'inv-1', HASH);
  });

  it('pays with Freighter and confirms in one step', async () => {
    vi.mocked(payInvoice.payInvoiceWithFreighter).mockResolvedValue(HASH);
    renderPanel();
    fireEvent.click(await screen.findByRole('button', { name: /Upgrade for 50 USDC/ }));
    fireEvent.click(await screen.findByRole('button', { name: /Pay with Freighter/ }));

    await waitFor(() => expect(apiKeys.confirmInvoice).toHaveBeenCalledWith('token', 'inv-1', HASH));
    expect(payInvoice.payInvoiceWithFreighter).toHaveBeenCalledWith(INVOICE, 'GWALLET');
  });

  it('shows the server message when a payment does not match', async () => {
    vi.mocked(apiKeys.confirmInvoice).mockRejectedValue(new Error('The transaction memo must be the text memo'));
    renderPanel();
    fireEvent.click(await screen.findByRole('button', { name: /Upgrade for 50 USDC/ }));
    fireEvent.change(await screen.findByLabelText('Transaction hash'), { target: { value: HASH } });
    fireEvent.click(screen.getByRole('button', { name: /Confirm payment/ }));

    expect(await screen.findByRole('alert')).toHaveTextContent('memo');
  });

  it('shows an active plan and offers renewal', async () => {
    vi.mocked(apiKeys.getSubscription).mockResolvedValue({
      api_key_id: 'key-1',
      plan: 'pro',
      limit_per_minute: 1000,
      paid_until: '2099-01-01T00:00:00Z',
    });
    renderPanel();
    expect(await screen.findByText(/on the Pro plan until/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Renew for 50 USDC/ })).toBeInTheDocument();
  });
});
