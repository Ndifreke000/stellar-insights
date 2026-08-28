import { renderHook, waitFor, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import {
  useSep24Anchors,
  useSep24Info,
  useSep24Transactions,
  useStartDepositFlow,
  useStartWithdrawFlow,
  useSep24FlowState,
} from '@/hooks/useSep24';
import { useAppStore } from '@/lib/zustand/store';

// Mock API functions
const mockGetSep24Anchors = async () => ({
  anchors: [
    {
      id: 'anchor-1',
      name: 'Test Anchor',
      transfer_server: 'https://anchor.example.com/sep24',
      country_code: 'US',
      languages: ['en'],
    },
  ],
});

const mockGetSep24Info = async (server: string) => ({
  deposit: {
    'USDC': {
      fees: { min: 0.5, max: 5 },
      enabled: true,
    },
  },
  withdraw: {
    'USD': {
      fees: { min: 0.5, max: 5 },
      enabled: true,
    },
  },
});

const mockGetSep24Transactions = async () => ({
  transactions: [
    {
      id: 'tx-1',
      kind: 'deposit',
      status: 'completed',
      amount_in: '100',
      amount_out: '100',
      asset_code: 'USDC',
    },
  ],
});

const mockStartDeposit = async () => ({
  url: 'https://anchor.example.com/deposit?token=test',
});

const mockStartWithdraw = async () => ({
  url: 'https://anchor.example.com/withdraw?token=test',
});

const createTestQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

const TestWrapper = ({ children }: { children: React.ReactNode }) => (
  <QueryClientProvider client={createTestQueryClient()}>
    {children}
  </QueryClientProvider>
);

describe('useSep24 Hooks', () => {
  beforeEach(() => {
    const store = useAppStore.getState();
    if (store.resetState) {
      store.resetState();
    }
  });

  describe('useSep24Anchors', () => {
    it('should fetch anchors successfully', async () => {
      const { result } = renderHook(() => useSep24Anchors(), {
        wrapper: TestWrapper,
      });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual(mockGetSep24Anchors().anchors);
    });

    it('should handle loading state', () => {
      const { result } = renderHook(() => useSep24Anchors(), {
        wrapper: TestWrapper,
      });

      expect(result.current.isLoading).toBe(true);
    });

    it('should handle errors', async () => {
      const { result } = renderHook(
        () => useSep24Anchors(),
        {
          wrapper: TestWrapper,
        }
      );

      await waitFor(() => expect(result.current.isError).toBe(true));
    });
  });

  describe('useSep24Info', () => {
    it('should fetch info when transfer server is provided', async () => {
      const { result } = renderHook(
        () => useSep24Info('https://anchor.example.com'),
        {
          wrapper: TestWrapper,
        }
      );

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual(mockGetSep24Info('https://anchor.example.com'));
    });

    it('should not fetch when transfer server is empty', () => {
      const { result } = renderHook(() => useSep24Info(''), {
        wrapper: TestWrapper,
      });

      expect(result.current.fetchStatus).toBe('idle');
    });

    it('should handle errors', async () => {
      const { result } = renderHook(
        () => useSep24Info('https://invalid.example.com'),
        {
          wrapper: TestWrapper,
        }
      );

      await waitFor(() => expect(result.current.isError).toBe(true));
    });
  });

  describe('useSep24Transactions', () => {
    it('should fetch transactions when transfer server is provided', async () => {
      const { result } = renderHook(
        () => useSep24Transactions('https://anchor.example.com', 'test-jwt'),
        {
          wrapper: TestWrapper,
        }
      );

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual(mockGetSep24Transactions().transactions);
    });

    it('should not fetch when transfer server is empty', () => {
      const { result } = renderHook(() => useSep24Transactions('', ''), {
        wrapper: TestWrapper,
      });

      expect(result.current.fetchStatus).toBe('idle');
    });
  });

  describe('useStartDepositFlow', () => {
    it('should start deposit flow successfully', async () => {
      const { result } = renderHook(() => useStartDepositFlow(), {
        wrapper: TestWrapper,
      });

      act(() => {
        result.current.mutate({
          transferServer: 'https://anchor.example.com',
          assetCode: 'USDC',
          amount: '100',
        });
      });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual(mockStartDeposit());
    });

    it('should handle errors', async () => {
      const { result } = renderHook(() => useStartDepositFlow(), {
        wrapper: TestWrapper,
      });

      act(() => {
        result.current.mutate({
          transferServer: 'https://invalid.example.com',
        });
      });

      await waitFor(() => expect(result.current.isError).toBe(true));
    });

    it('should add success notification on success', async () => {
      const { result } = renderHook(() => useStartDepositFlow(), {
        wrapper: TestWrapper,
      });
      
      act(() => {
        result.current.mutate({
          transferServer: 'https://anchor.example.com',
          assetCode: 'USDC',
        });
      });
      
      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      
      const notifications = useAppStore.getState().notifications;
      expect(notifications).toHaveLength(1);
      expect(notifications[0].type).toBe('success');
    });
  });

  describe('useStartWithdrawFlow', () => {
    it('should start withdraw flow successfully', async () => {
      const { result } = renderHook(() => useStartWithdrawFlow(), {
        wrapper: TestWrapper,
      });

      act(() => {
        result.current.mutate({
          transferServer: 'https://anchor.example.com',
          assetCode: 'USD',
          amount: '100',
        });
      });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual(mockStartWithdraw());
    });

    it('should add success notification on success', async () => {
      const { result } = renderHook(() => useStartWithdrawFlow(), {
        wrapper: TestWrapper,
      });

      act(() => {
        result.current.mutate({
          transferServer: 'https://anchor.example.com',
          assetCode: 'USD',
        });
      });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));

      const notifications = useAppStore.getState().notifications;
      expect(notifications).toHaveLength(1);
      expect(notifications[0].type).toBe('success');
    });
  });

  describe('useSep24FlowState', () => {
    it('should manage form data', () => {
      const { result } = renderHook(() => useSep24FlowState());
      
      act(() => {
        result.current.setFormDataValue('field1', 'value1');
      });
      
      const store = useAppStore.getState();
      expect(store.formData['sep24-flow']).toEqual({
        field1: 'value1',
      });
    });

    it('should clear form data', () => {
      const { result } = renderHook(() => useSep24FlowState());
      
      act(() => {
        result.current.setFormDataValue('field1', 'value1');
        result.current.setFormDataValue('field2', 'value2');
      });
      
      act(() => {
        result.current.clearFormDataValue('field1');
      });
      
      const store = useAppStore.getState();
      expect(store.formData['sep24-flow']).toEqual({
        field2: 'value2',
      });
    });

    it('should manage loading states', () => {
      const { result } = renderHook(() => useSep24FlowState());
      
      act(() => {
        result.current.setLoading('test-loading', true);
      });
      
      const store = useAppStore.getState();
      expect(store.loading['test-loading']).toBe(true);
      
      act(() => {
        result.current.clearLoading('test-loading');
      });
      
      expect(store.loading['test-loading']).toBeUndefined();
    });
  });

  describe('Integration Tests', () => {
    it('should work together - form state + mutations', async () => {
      const { result: flowState } = renderHook(() => useSep24FlowState());
      const { result: depositMutation } = renderHook(() => useStartDepositFlow(), {
        wrapper: TestWrapper,
      });

      // Set form data
      act(() => {
        flowState.current.setFormDataValue('transferServer', 'https://anchor.example.com');
        flowState.current.setFormDataValue('assetCode', 'USDC');
      });

      // Start deposit flow
      act(() => {
        depositMutation.current.mutate({
          transferServer: 'https://anchor.example.com',
          assetCode: 'USDC',
          amount: '100',
        });
      });

      await waitFor(() => expect(depositMutation.current.isSuccess).toBe(true));

      // Check that notifications were added
      const notifications = useAppStore.getState().notifications;
      expect(notifications).toHaveLength(1);
      expect(notifications[0].type).toBe('success');
    });

    it('should handle form errors', async () => {
      const { result: flowState } = renderHook(() => useSep24FlowState());
      
      act(() => {
        flowState.current.setFormErrors('sep24-flow', {
          transferServer: 'Invalid URL',
          assetCode: 'Required',
        });
      });
      
      const store = useAppStore.getState();
      expect(store.formErrors['sep24-flow']).toEqual({
        transferServer: 'Invalid URL',
        assetCode: 'Required',
      });
      
      act(() => {
        flowState.current.clearFormErrors('sep24-flow');
      });
      
      expect(store.formErrors['sep24-flow']).toEqual({});
    });
  });
});
