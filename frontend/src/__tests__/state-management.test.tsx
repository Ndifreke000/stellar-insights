import { renderHook, waitFor, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { z } from 'zod';

import { useAppStore } from '@/lib/zustand/store';
import { queryKeys } from '@/lib/react-query/hooks';
import { useApiQuery, useApiMutation } from '@/lib/react-query/hooks';

// Mock API functions for testing
const mockFetchAnchors = async () => ({
  anchors: [
    {
      id: 'test-anchor-1',
      name: 'Test Anchor 1',
      transfer_server: 'https://anchor1.example.com/sep24',
      country_code: 'US',
      languages: ['en'],
    },
    {
      id: 'test-anchor-2',
      name: 'Test Anchor 2',
      transfer_server: 'https://anchor2.example.com/sep24',
      country_code: 'GB',
      languages: ['en', 'es'],
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
    'GBP': {
      fees: { min: 0.5, max: 5 },
      enabled: true,
    },
  },
});

interface MockSep24Request {
  transfer_server: string;
  asset_code: string;
  amount: string;
  account: string;
  jwt: string;
}

const mockStartDeposit = async ({ transfer_server, asset_code, amount, account, jwt }: MockSep24Request) => ({
  url: `https://anchor.example.com/deposit?token=${jwt}`,
  id: 'test-transaction-id',
});

const mockStartWithdraw = async ({ transfer_server, asset_code, amount, account, jwt }: MockSep24Request) => ({
  id: 'test-transaction-id',
  url: `https://anchor.example.com/withdraw?token=${jwt}`,
});

// Mock query client for testing
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

describe('State Management', () => {
  beforeEach(() => {
    // Reset store before each test
    const store = useAppStore.getState();
    if (store.resetState) {
      store.resetState();
    }
  });

  describe('Zustand Store', () => {
    it('should initialize with default values', () => {
      const store = useAppStore.getState();

      expect(store.sidebarCollapsed).toBe(false);
      expect(store.activeModal).toBe(null);
      expect(store.formData).toEqual({});
      expect(store.formErrors).toEqual({});
      expect(store.loading).toEqual({});
    });

    it('should update sidebar state', () => {
      const { setSidebarCollapsed } = useAppStore.getState();

      act(() => {
        setSidebarCollapsed(true);
      });

      const store = useAppStore.getState();
      expect(store.sidebarCollapsed).toBe(true);
    });

    it('should manage form data', () => {
      const { setFormDataValue, clearFormDataValue } = useAppStore.getState();

      act(() => {
        setFormDataValue('test-form', { field1: 'value1', field2: 'value2' });
      });

      const store = useAppStore.getState();
      expect(store.formData['test-form']).toEqual({
        field1: 'value1',
        field2: 'value2',
      });

      act(() => {
        clearFormDataValue('field1');
      });

      const updatedStore = useAppStore.getState();
      expect(updatedStore.formData['test-form']).toEqual({
        field2: 'value2',
      });
    });

    it('should handle notifications', () => {
      const { addNotification } = useAppStore.getState();

      act(() => {
        addNotification({
          type: 'success',
          message: 'Test notification',
        });
      });

      const store = useAppStore.getState();
      expect(store.notifications).toHaveLength(1);
      expect(store.notifications[0]).toMatchObject({
        type: 'success',
        message: 'Test notification',
        timestamp: expect.any(Number),
        read: false,
        id: expect.any(String),
      });
    });
  });

  describe('React Query', () => {
    it('should cache API responses', async () => {
      const { result } = renderHook(
        () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
        {
          wrapper: TestWrapper,
        }
      );

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual(mockFetchAnchors().anchors);
    });

    it('should handle loading states', async () => {
      const { result, isLoading } = renderHook(
        () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
        {
          wrapper: TestWrapper,
        }
      );

      expect(isLoading).toBe(true);

      await waitFor(() => expect(isLoading).toBe(false));
    });

    it('should handle errors', async () => {
      const mockError = new Error('API Error');
      const mockFetchAnchors = async () => {
        throw mockError;
      };

      const { result } = renderHook(
        () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
        {
          wrapper: TestWrapper,
        }
      );

      await waitFor(() => expect(result.current.isError).toBe(true));
      expect(result.error?.message).toBe('API Error');
    });

    it('should invalidate queries', async () => {
      const { result, refetch } = renderHook(
        () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
        {
          wrapper: TestWrapper,
        }
      );

      // Initial load
      await waitFor(() => expect(result.current.isSuccess).toBe(true);
      expect(result.current.data).toEqual(mockFetchAnchors().anchors);

      // Invalidate and refetch
      act(() => {
        refetch();
      });

      await waitFor(() => expect(result.current.isFetching).toBe(true);
      await waitFor(() => expect(result.current.isSuccess).toBe(true);
    });

    it('should handle mutations', async () => {
      const { mutate, mutateAsync } = useApiMutation(mockStartDeposit, {
        onSuccess: () => {
          const store = useAppStore.getState();
          expect(store.notifications).toHaveLength(0);
        },
      });

      const { result } = renderHook(() => mutateAsync({
        transfer_server: 'https://anchor.example.com',
        asset_code: 'USDC',
        amount: '100',
      }), {
        wrapper: TestWrapper,
      });

      await waitFor(() => result.current.isSuccess).toBe(true);
      expect(result.data).toEqual({
        id: 'test-transaction-id',
        url: 'https://anchor.example.com/deposit?token=undefined',
      });
    });
  });

  it('should prefetch data', async () => {
    const { prefetch } = usePrefetchQuery();

    act(() => {
      prefetch(queryKeys.anchors, mockFetchAnchors);
    });

    // Prefetching doesn't return data, but should trigger background fetch
    await new Promise(resolve => setTimeout(resolve, 100));

    const { result } = renderHook(
      () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
      {
        wrapper: TestWrapper,
      }
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true);
    expect(result.current.data).toEqual(mockFetchAnchors().anchors);
  });
});

it('should handle query invalidation', async () => {
  const { result } = renderHook(
    () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
    {
      wrapper: TestWrapper,
    }
  );

  await waitFor(() => expect(result.current.isSuccess).toBe(true);

  // Test with invalid data
  const invalidQuery = renderHook(
    () => useApiQuery(
      queryKeys.anchors,
      async () => {
        throw new Error('Invalid query');
      },
      {
        wrapper: TestWrapper,
      }
    )
  );

  await waitFor(() => expect(invalidQuery.current.isError).toBe(true);
  expect(invalidQuery.error?.message).toBe('Invalid query');
});
  });

describe('Integration', () => {
  it('should work together - React Query + Zustand', async () => {
    const { result: anchors } = renderHook(() => useApiQuery(queryKeys.anchors, mockFetchAnchors));

    const { addNotification, setFormDataValue } = useAppStore();

    // Simulate user interaction
    act(() => {
      setFormDataValue('test', { transferServer: 'https://anchor.example.com' });
    });

    await waitFor(() => expect(anchors.isSuccess).toBe(true));

    act(() => {
      addNotification({
        type: 'info',
        message: 'Form submitted successfully',
      });
    });

    const store = useAppStore.getState();
    expect(store.formData.test).toEqual({
      transferServer: 'https://anchor.example.com',
    });

    const notifications = store.notifications;
    expect(notifications).toHaveLength(1);
    expect(notifications[0].type).toBe('info');
  });
});
});

// Add React Query tests
describe('React Query Integration', () => {
  beforeEach(() => {
    const queryClient = createTestQueryClient();
    queryClient.clear();
  });

  it('should persist QueryClient across renders', async () => {
    const { result: result1 } = renderHook(
      () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
      {
        wrapper: TestWrapper,
      }
    );

    await waitFor(() => expect(result1.current.isSuccess).toBe(true));

    const { result: result2 } = renderHook(
      () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
      {
        wrapper: TestWrapper,
      }
    );

    // Should use same cache
    expect(result1.current.data).toEqual(result2.current.data);
  });

  it('should handle query invalidation in Zustand', async () => {
    const { addNotification } = useAppStore();

    const { result } = renderHook(
      () => useApiQuery(
        queryKeys.anchors,
        async () => {
          throw new Error('API Error');
        },
        {
          wrapper: TestWrapper,
        }
      )
      
      await waitFor(() => expect(result.current.isError).toBe(true));
    expect(addNotification).toHaveBeenCalledTimes(1);

    const notifications = useAppStore.getState().notifications;
    expect(notifications[0].type).toBe('error');
  });
});
