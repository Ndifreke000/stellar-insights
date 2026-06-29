import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { NetworkSwitcher } from '@/components/NetworkSwitcher';

const MAINNET_NETWORK = {
  network: 'mainnet' as const,
  display_name: 'Mainnet',
  rpc_url: 'https://stellar.api.onfinality.io/public',
  horizon_url: 'https://horizon.stellar.org',
  network_passphrase: 'Public Global Stellar Network ; September 2015',
  color: '#2563EB',
  is_mainnet: true,
  is_testnet: false,
};

const TESTNET_NETWORK = {
  network: 'testnet' as const,
  display_name: 'Testnet',
  rpc_url: 'https://soroban-testnet.stellar.org',
  horizon_url: 'https://horizon-testnet.stellar.org',
  network_passphrase: 'Test SDF Network ; September 2015',
  color: '#4ECDC4',
  is_mainnet: false,
  is_testnet: true,
};

function renderWithQueryClient(ui: React.ReactElement, queryClient: QueryClient) {
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
}

describe('NetworkSwitcher', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn((url: string) => {
      if (url.includes('/api/network/info')) {
        return Promise.resolve({
          ok: true,
          json: async () => MAINNET_NETWORK,
        });
      }
      if (url.includes('/api/network/available')) {
        return Promise.resolve({
          ok: true,
          json: async () => [MAINNET_NETWORK, TESTNET_NETWORK],
        });
      }
      if (url.includes('/api/network/switch')) {
        return Promise.resolve({
          ok: true,
          json: async () => ({ message: 'Network switched to testnet.' }),
        });
      }
      return Promise.reject(new Error(`Unhandled fetch: ${url}`));
    }));

    vi.stubGlobal('alert', vi.fn());
  });

  it('clears React Query cache when network switch is confirmed', async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const clearSpy = vi.spyOn(queryClient, 'clear');

    queryClient.setQueryData(['anchors'], [{ id: 'stale-mainnet' }]);

    renderWithQueryClient(<NetworkSwitcher />, queryClient);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Network: Mainnet/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /Network: Mainnet/i }));
    fireEvent.click(screen.getByRole('option', { name: /Testnet/i }));
    fireEvent.click(screen.getByLabelText('Confirm switch to Testnet'));

    await waitFor(() => {
      expect(clearSpy).toHaveBeenCalledTimes(1);
    });

    expect(queryClient.getQueryData(['anchors'])).toBeUndefined();
  });
});
