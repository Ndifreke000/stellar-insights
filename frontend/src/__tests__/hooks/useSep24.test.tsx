import type { ReactNode } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  useSep24Anchors,
  useSep24FlowState,
  useSep24Info,
  useSep24Transactions,
  useStartDepositFlow,
  useStartWithdrawFlow,
} from '@/hooks/useSep24';
import { useAppStore } from '@/lib/zustand/store';
import * as sep24 from '@/services/sep24';

vi.mock('@/services/sep24');

const TRANSFER_SERVER = 'https://anchor.example.com/sep24';

const ANCHORS = {
  anchors: [{ name: 'Test Anchor', transfer_server: TRANSFER_SERVER }],
};
const INFO = {
  deposit: { USDC: { enabled: true } },
  withdraw: { USDC: { enabled: true } },
};
const TRANSACTIONS = {
  transactions: [{ id: 'tx-1', kind: 'deposit', status: 'completed' }],
};

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

const notifications = () => useAppStore.getState().notifications;

describe('useSep24 hooks', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.getState().resetState();
    vi.mocked(sep24.getSep24Anchors).mockResolvedValue(ANCHORS as never);
    vi.mocked(sep24.getSep24Info).mockResolvedValue(INFO as never);
    vi.mocked(sep24.getSep24Transactions).mockResolvedValue(TRANSACTIONS as never);
    vi.spyOn(window, 'open').mockReturnValue(null);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('useSep24Anchors', () => {
    it('returns the anchors from the service', async () => {
      const { result } = renderHook(() => useSep24Anchors(), { wrapper });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual(ANCHORS);
    });

    it('starts in the loading state', () => {
      const { result } = renderHook(() => useSep24Anchors(), { wrapper });
      expect(result.current.isLoading).toBe(true);
    });
  });

  describe('useSep24Info', () => {
    it('does not fetch until a transfer server is given', () => {
      const { result } = renderHook(() => useSep24Info(''), { wrapper });

      expect(result.current.fetchStatus).toBe('idle');
      expect(sep24.getSep24Info).not.toHaveBeenCalled();
    });

    it('fetches info for the given transfer server', async () => {
      const { result } = renderHook(() => useSep24Info(TRANSFER_SERVER), { wrapper });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(sep24.getSep24Info).toHaveBeenCalledWith(TRANSFER_SERVER);
      expect(result.current.data).toEqual(INFO);
    });
  });

  describe('useSep24Transactions', () => {
    it('does not fetch without a transfer server', () => {
      renderHook(() => useSep24Transactions(''), { wrapper });
      expect(sep24.getSep24Transactions).not.toHaveBeenCalled();
    });

    it('passes the transfer server, token and page size to the service', async () => {
      const { result } = renderHook(() => useSep24Transactions(TRANSFER_SERVER, 'jwt-token'), {
        wrapper,
      });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(sep24.getSep24Transactions).toHaveBeenCalledWith({
        transfer_server: TRANSFER_SERVER,
        jwt: 'jwt-token',
        limit: 20,
      });
      expect(result.current.data).toEqual(TRANSACTIONS);
    });
  });

  describe('useStartDepositFlow', () => {
    it('opens the interactive URL and notifies on success', async () => {
      vi.mocked(sep24.startDepositInteractive).mockResolvedValue({
        url: 'https://anchor.example.com/deposit?token=test',
      } as never);
      const { result } = renderHook(() => useStartDepositFlow(), { wrapper });

      act(() => {
        result.current.mutate({ transferServer: TRANSFER_SERVER, assetCode: 'USDC', amount: '100' });
      });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(sep24.startDepositInteractive).toHaveBeenCalledWith(
        expect.objectContaining({ transfer_server: TRANSFER_SERVER, asset_code: 'USDC', amount: '100' }),
      );
      expect(window.open).toHaveBeenCalledWith(
        'https://anchor.example.com/deposit?token=test',
        'sep24-interactive',
        expect.any(String),
      );
      expect(notifications()[0]).toMatchObject({ type: 'success' });
    });

    it('does not open a URL that is not http(s)', async () => {
      vi.mocked(sep24.startDepositInteractive).mockResolvedValue({
        url: 'javascript:alert(1)',
      } as never);
      const { result } = renderHook(() => useStartDepositFlow(), { wrapper });

      act(() => {
        result.current.mutate({ transferServer: TRANSFER_SERVER, assetCode: 'USDC' });
      });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(window.open).not.toHaveBeenCalled();
    });

    it('notifies with the failure reason on error', async () => {
      vi.mocked(sep24.startDepositInteractive).mockRejectedValue(new Error('anchor unavailable'));
      const { result } = renderHook(() => useStartDepositFlow(), { wrapper });

      act(() => {
        result.current.mutate({ transferServer: TRANSFER_SERVER, assetCode: 'USDC' });
      });

      await waitFor(() => expect(result.current.isError).toBe(true));
      expect(window.open).not.toHaveBeenCalled();
      expect(notifications()[0]).toMatchObject({ type: 'error' });
      expect(notifications()[0].message).toContain('anchor unavailable');
    });
  });

  describe('useStartWithdrawFlow', () => {
    it('opens the interactive URL and notifies on success', async () => {
      vi.mocked(sep24.startWithdrawInteractive).mockResolvedValue({
        url: 'https://anchor.example.com/withdraw?token=test',
      } as never);
      const { result } = renderHook(() => useStartWithdrawFlow(), { wrapper });

      act(() => {
        result.current.mutate({ transferServer: TRANSFER_SERVER, assetCode: 'USDC' });
      });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(window.open).toHaveBeenCalledWith(
        'https://anchor.example.com/withdraw?token=test',
        'sep24-interactive',
        expect.any(String),
      );
      expect(notifications()[0]).toMatchObject({ type: 'success' });
    });

    it('notifies with the failure reason on error', async () => {
      vi.mocked(sep24.startWithdrawInteractive).mockRejectedValue(new Error('limit exceeded'));
      const { result } = renderHook(() => useStartWithdrawFlow(), { wrapper });

      act(() => {
        result.current.mutate({ transferServer: TRANSFER_SERVER, assetCode: 'USDC' });
      });

      await waitFor(() => expect(result.current.isError).toBe(true));
      expect(notifications()[0].message).toContain('limit exceeded');
    });
  });

  describe('useSep24FlowState', () => {
    it('starts with empty form data', () => {
      const { result } = renderHook(() => useSep24FlowState());
      expect(result.current.formData).toEqual({});
    });

    it('sets and clears individual form values', () => {
      const { result } = renderHook(() => useSep24FlowState());

      act(() => result.current.setFormDataValue('assetCode', 'USDC'));
      act(() => result.current.setFormDataValue('amount', '100'));
      expect(result.current.formData).toEqual({ assetCode: 'USDC', amount: '100' });

      act(() => result.current.clearFormDataValue('amount'));
      expect(result.current.formData).toEqual({ assetCode: 'USDC' });

      act(() => result.current.clearFormDataValue());
      expect(result.current.formData).toEqual({});
    });

    it('stores and clears form errors in the app store', () => {
      const { result } = renderHook(() => useSep24FlowState());
      const errors = { transferServer: 'Invalid URL', assetCode: 'Required' };

      act(() => result.current.setFormErrors('sep24-flow', errors));
      expect(useAppStore.getState().formErrors['sep24-flow']).toEqual(errors);

      act(() => result.current.clearFormErrors('sep24-flow'));
      expect(useAppStore.getState().formErrors['sep24-flow']).toBeUndefined();
    });

    it('exposes loading flags from the app store', () => {
      const { result } = renderHook(() => useSep24FlowState());

      act(() => result.current.setLoading('sep24-start', true));
      expect(result.current.loading['sep24-start']).toBe(true);

      act(() => result.current.setLoading('sep24-start', false));
      expect(result.current.loading['sep24-start']).toBeFalsy();
    });
  });
});
