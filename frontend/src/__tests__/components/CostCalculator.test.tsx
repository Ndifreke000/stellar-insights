/**
 * Functional tests for the CostCalculator component.
 *
 * Tests cover:
 * - Initial render state
 * - Form field interactions (currency selects, amounts, route checkboxes)
 * - Validation (canSubmit guard)
 * - Successful calculation flow (mock fetch)
 * - Error handling (network errors, API errors)
 * - Route toggle behaviour
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';
import { CostCalculator } from '@/components/CostCalculator';

// ── mocks ─────────────────────────────────────────────────────────────────────

const SUCCESS_RESPONSE = {
  source_currency: 'USDC',
  destination_currency: 'NGN',
  source_amount: 1000,
  source_usd_rate: 1.0,
  destination_usd_rate: 0.00065,
  mid_market_rate: 1538.46,
  best_route: {
    route: 'stellar_dex',
    route_name: 'Stellar DEX',
    breakdown: {
      exchange_rate_mid: 1538.46,
      effective_rate: 1530.0,
      spread_bps: 10,
      slippage_bps: 5,
      spread_cost_source: 1.0,
      service_fee_source: 0.5,
      network_fee_source: 0.01,
      slippage_cost_source: 0.5,
      total_fees_source: 2.01,
      total_fees_destination: 3090.0,
      estimated_destination_amount: 1_530_000,
    },
  },
  routes: [
    {
      route: 'stellar_dex' as const,
      route_name: 'Stellar DEX',
      breakdown: {
        exchange_rate_mid: 1538.46,
        effective_rate: 1530.0,
        spread_bps: 10,
        slippage_bps: 5,
        spread_cost_source: 1.0,
        service_fee_source: 0.5,
        network_fee_source: 0.01,
        slippage_cost_source: 0.5,
        total_fees_source: 2.01,
        total_fees_destination: 3090.0,
        estimated_destination_amount: 1_530_000,
      },
    },
  ],
  timestamp: new Date().toISOString(),
};

// Re-assign before every test so stale mockResolvedValueOnce queues never bleed
// across tests (vi.clearAllMocks only clears call history, not implementations).
beforeEach(() => {
  vi.resetAllMocks();
  global.fetch = vi.fn();
});

// ── initial render ────────────────────────────────────────────────────────────

describe('CostCalculator – initial render', () => {
  it('renders the form heading', () => {
    render(<CostCalculator />);
    expect(screen.getByRole('button', { name: /calculate total cost/i })).toBeInTheDocument();
  });

  it('shows USDC as default source currency', () => {
    render(<CostCalculator />);
    const sourceSelect = screen.getAllByRole('combobox')[0];
    expect((sourceSelect as HTMLSelectElement).value).toBe('USDC');
  });

  it('shows NGN as default destination currency', () => {
    render(<CostCalculator />);
    const destSelect = screen.getAllByRole('combobox')[1];
    expect((destSelect as HTMLSelectElement).value).toBe('NGN');
  });

  it('shows 1000 as default source amount', () => {
    render(<CostCalculator />);
    // The source-amount input has value="1000" by default
    expect(screen.getByDisplayValue('1000')).toBeInTheDocument();
  });

  it('renders all three route checkboxes', () => {
    render(<CostCalculator />);
    expect(screen.getByLabelText(/stellar dex/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/anchor direct/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/liquidity pool/i)).toBeInTheDocument();
  });

  it('all route checkboxes are checked by default', () => {
    render(<CostCalculator />);
    const checkboxes = screen.getAllByRole('checkbox');
    checkboxes.forEach((cb) => {
      expect(cb).toBeChecked();
    });
  });

  it('submit button is enabled with valid defaults', () => {
    render(<CostCalculator />);
    const button = screen.getByRole('button', { name: /calculate total cost/i });
    expect(button).not.toBeDisabled();
  });

  it('no error message shown initially', () => {
    render(<CostCalculator />);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('no result panel shown initially', () => {
    render(<CostCalculator />);
    expect(screen.queryByText(/mid-market rate/i)).not.toBeInTheDocument();
  });
});

// ── input validation & canSubmit ─────────────────────────────────────────────

describe('CostCalculator – canSubmit guard', () => {
  it('disables submit when amount is 0', async () => {
    render(<CostCalculator />);
    // The source-amount input carries the current numeric value
    const input = screen.getByDisplayValue('1000');
    fireEvent.change(input, { target: { value: '0' } });
    expect(screen.getByRole('button', { name: /calculate total cost/i })).toBeDisabled();
  });

  it('disables submit when amount is empty', async () => {
    render(<CostCalculator />);
    const input = screen.getByDisplayValue('1000');
    fireEvent.change(input, { target: { value: '' } });
    expect(screen.getByRole('button', { name: /calculate total cost/i })).toBeDisabled();
  });

  it('disables submit when amount is negative', async () => {
    render(<CostCalculator />);
    const input = screen.getByDisplayValue('1000');
    fireEvent.change(input, { target: { value: '-100' } });
    expect(screen.getByRole('button', { name: /calculate total cost/i })).toBeDisabled();
  });

  it('disables submit when no routes are selected', async () => {
    render(<CostCalculator />);
    const checkboxes = screen.getAllByRole('checkbox');
    // Uncheck all
    for (const cb of checkboxes) {
      fireEvent.click(cb);
    }
    expect(screen.getByRole('button', { name: /calculate total cost/i })).toBeDisabled();
  });

  it('re-enables submit when a route is checked again', async () => {
    render(<CostCalculator />);
    const checkboxes = screen.getAllByRole('checkbox');
    // Uncheck all
    for (const cb of checkboxes) {
      fireEvent.click(cb);
    }
    // Re-check the first one
    fireEvent.click(checkboxes[0]);
    expect(screen.getByRole('button', { name: /calculate total cost/i })).not.toBeDisabled();
  });
});

// ── route toggle ──────────────────────────────────────────────────────────────

describe('CostCalculator – route toggle', () => {
  it('unchecks a route when its checkbox is clicked', async () => {
    render(<CostCalculator />);
    const stellarDex = screen.getByLabelText(/stellar dex/i);
    fireEvent.click(stellarDex);
    expect(stellarDex).not.toBeChecked();
  });

  it('re-checks a route after two clicks', async () => {
    render(<CostCalculator />);
    const stellarDex = screen.getByLabelText(/stellar dex/i);
    fireEvent.click(stellarDex);
    fireEvent.click(stellarDex);
    expect(stellarDex).toBeChecked();
  });
});

// ── successful calculation ────────────────────────────────────────────────────

describe('CostCalculator – successful calculation', () => {
  beforeEach(() => {
    (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
      ok: true,
      json: async () => SUCCESS_RESPONSE,
    });
  });

  it('calls fetch with POST and correct URL', async () => {
    render(<CostCalculator />);
    const form = screen.getByRole('button', { name: /calculate/i }).closest('form')!;
    fireEvent.submit(form);

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/cost-calculator/estimate'),
        expect.objectContaining({ method: 'POST' }),
      );
    });
  });

  it('shows loading indicator while request is in flight', async () => {
    let resolveRequest!: (v: unknown) => void;
    (global.fetch as ReturnType<typeof vi.fn>).mockReturnValueOnce(
      new Promise((res) => { resolveRequest = res; }),
    );

    render(<CostCalculator />);
    fireEvent.submit(
      screen.getByRole('button', { name: /calculate/i }).closest('form')!,
    );

    expect(await screen.findByText(/calculating/i)).toBeInTheDocument();

    resolveRequest({ ok: true, json: async () => SUCCESS_RESPONSE });
  });

  it('renders result summary after success', async () => {
    render(<CostCalculator />);
    fireEvent.submit(
      screen.getByRole('button', { name: /calculate/i }).closest('form')!,
    );

    await waitFor(() => {
      expect(screen.getByText(/mid-market rate/i)).toBeInTheDocument();
    });
  });

  it('shows the best route name in the result summary', async () => {
    render(<CostCalculator />);
    fireEvent.submit(
      screen.getByRole('button', { name: /calculate/i }).closest('form')!,
    );

    await waitFor(() => {
      expect(screen.getByText(/stellar dex/i)).toBeInTheDocument();
    });
  });
});

// ── error handling ────────────────────────────────────────────────────────────

describe('CostCalculator – error handling', () => {
  it('shows error when fetch rejects (network failure)', async () => {
    (global.fetch as ReturnType<typeof vi.fn>).mockRejectedValueOnce(
      new Error('Network error'),
    );

    render(<CostCalculator />);
    fireEvent.submit(
      screen.getByRole('button', { name: /calculate/i }).closest('form')!,
    );

    await waitFor(() => {
      expect(screen.getByText(/network error/i)).toBeInTheDocument();
    });
  });

  it('shows error when API returns a non-OK status', async () => {
    (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
      ok: false,
      json: async () => ({ error: 'Invalid currency pair' }),
    });

    render(<CostCalculator />);
    fireEvent.submit(
      screen.getByRole('button', { name: /calculate/i }).closest('form')!,
    );

    await waitFor(() => {
      expect(screen.getByText(/invalid currency pair/i)).toBeInTheDocument();
    });
  });

  it('clears previous result when a new error occurs', async () => {
    // First call succeeds
    (global.fetch as ReturnType<typeof vi.fn>)
      .mockResolvedValueOnce({ ok: true, json: async () => SUCCESS_RESPONSE })
      // Second call fails
      .mockRejectedValueOnce(new Error('Server down'));

    render(<CostCalculator />);

    const form = screen.getByRole('button', { name: /calculate/i }).closest('form')!;

    // First submit
    fireEvent.submit(form);
    await waitFor(() => screen.getByText(/mid-market rate/i));

    // Second submit
    fireEvent.submit(form);
    await waitFor(() => {
      expect(screen.queryByText(/mid-market rate/i)).not.toBeInTheDocument();
    });
  });
});
