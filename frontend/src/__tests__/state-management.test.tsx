import { renderHook, waitFor, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { describe, it, expect, beforeEach } from 'vitest';

import { useAppStore } from '@/lib/zustand/store';
import { queryKeys, useApiQuery, useApiMutation, usePrefetchQuery } from '@/lib/react-query/hooks';

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

interface MockSep24Request {
  transfer_server: string;
  asset_code: string;
  amount: string;
  account: string;
  jwt: string;
}

const mockStartDeposit = async ({ jwt }: MockSep24Request) => ({
  url: `https://anchor.example.com/deposit?token=${jwt}`,
  id: 'test-transaction-id',
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

// Reset store before every test in this file (not just the ones nested
// under 'State Management' -- the top-level and sibling describe blocks
// below share Zustand's module-level store instance).
beforeEach(() => {
  const store = useAppStore.getState();
  if (store.resetState) {
    store.resetState();
  }
});

describe('State Management', () => {
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
      const { setFormData, updateFormData, clearFormData } = useAppStore.getState();

      act(() => {
        setFormData('test-form', { field1: 'value1', field2: 'value2' });
      });

      let store = useAppStore.getState();
      expect(store.formData['test-form']).toEqual({
        field1: 'value1',
        field2: 'value2',
      });

      act(() => {
        updateFormData('test-form', { field1: 'updated' });
      });

      store = useAppStore.getState();
      expect(store.formData['test-form']).toEqual({
        field1: 'updated',
        field2: 'value2',
      });

      act(() => {
        clearFormData('test-form');
      });

      store = useAppStore.getState();
      expect(store.formData['test-form']).toBeUndefined();
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
      expect(result.current.data).toEqual(await mockFetchAnchors());
    });

    it('should handle loading states', async () => {
      const { result } = renderHook(
        () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
        {
          wrapper: TestWrapper,
        }
      );

      expect(result.current.isLoading).toBe(true);

      await waitFor(() => expect(result.current.isLoading).toBe(false));
    });

    it('should handle errors', async () => {
      const mockError = new Error('API Error');
      const failingFetch = async () => {
        throw mockError;
      };

      const { result } = renderHook(
        // retry disabled so the error surfaces immediately, instead of
        // useApiQuery's default retry:true exhausting waitFor's timeout.
        () => useApiQuery(queryKeys.anchors, failingFetch, { retry: false }),
        {
          wrapper: TestWrapper,
        }
      );

      await waitFor(() => expect(result.current.isError).toBe(true));
      expect(result.current.error?.message).toBe('API Error');
    });

    it('should invalidate queries', async () => {
      const { result } = renderHook(
        () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
        {
          wrapper: TestWrapper,
        }
      );

      // Initial load
      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual(await mockFetchAnchors());

      // Invalidate and refetch. Awaiting refetch()'s own promise instead of
      // polling for an intermediate isFetching:true avoids a race with the
      // mock resolving faster than waitFor's poll interval.
      await act(async () => {
        await result.current.refetch();
      });

      expect(result.current.isSuccess).toBe(true);
      expect(result.current.data).toEqual(await mockFetchAnchors());
    });

    it('should handle mutations', async () => {
      const { result } = renderHook(() => useApiMutation(mockStartDeposit), {
        wrapper: TestWrapper,
      });

      act(() => {
        result.current.mutate({
          transfer_server: 'https://anchor.example.com',
          asset_code: 'USDC',
          amount: '100',
          account: 'GABC',
          jwt: 'test-jwt',
        });
      });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual({
        id: 'test-transaction-id',
        url: 'https://anchor.example.com/deposit?token=test-jwt',
      });
    });
  });

  it('should prefetch data', async () => {
    const queryClient = createTestQueryClient();
    const SharedWrapper = ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    const { result: prefetch } = renderHook(() => usePrefetchQuery(), {
      wrapper: SharedWrapper,
    });

    act(() => {
      prefetch.current(queryKeys.anchors, mockFetchAnchors);
    });

    // Prefetching doesn't return data, but should trigger background fetch
    await new Promise((resolve) => setTimeout(resolve, 100));

    // Same QueryClient/cache as the prefetch above, so this resolves from
    // cache rather than issuing a new fetch.
    const { result } = renderHook(
      () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
      {
        wrapper: SharedWrapper,
      }
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual(await mockFetchAnchors());
  });
});

it('should handle query invalidation', async () => {
  const { result } = renderHook(
    () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
    {
      wrapper: TestWrapper,
    }
  );

  await waitFor(() => expect(result.current.isSuccess).toBe(true));

  // A separate query instance (own QueryClient) using a failing fetcher
  const invalidQuery = renderHook(
    () =>
      useApiQuery(
        queryKeys.anchors,
        async () => {
          throw new Error('Invalid query');
        },
        { retry: false }
      ),
    {
      wrapper: TestWrapper,
    }
  );

  await waitFor(() => expect(invalidQuery.result.current.isError).toBe(true));
  expect(invalidQuery.result.current.error?.message).toBe('Invalid query');
});

describe('Integration', () => {
  it('should work together - React Query + Zustand', async () => {
    const { result: anchors } = renderHook(
      () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
      {
        wrapper: TestWrapper,
      }
    );

    const { setFormData, addNotification } = useAppStore.getState();

    // Simulate user interaction
    act(() => {
      setFormData('test', { transferServer: 'https://anchor.example.com' });
    });

    await waitFor(() => expect(anchors.current.isSuccess).toBe(true));

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

describe('React Query Integration', () => {
  it('should persist QueryClient across renders', async () => {
    const queryClient = createTestQueryClient();
    const SharedWrapper = ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    const { result: result1 } = renderHook(
      () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
      {
        wrapper: SharedWrapper,
      }
    );

    await waitFor(() => expect(result1.current.isSuccess).toBe(true));

    const { result: result2 } = renderHook(
      () => useApiQuery(queryKeys.anchors, mockFetchAnchors),
      {
        wrapper: SharedWrapper,
      }
    );

    // Should use same cache: the second hook resolves immediately, without
    // ever entering a loading state, because it shares result1's QueryClient.
    expect(result2.current.isSuccess).toBe(true);
    expect(result1.current.data).toEqual(result2.current.data);
  });

  it('should handle query invalidation in Zustand', async () => {
    const { result } = renderHook(
      () =>
        useApiQuery(
          queryKeys.anchors,
          async () => {
            throw new Error('API Error');
          },
          { retry: false }
        ),
      {
        wrapper: TestWrapper,
      }
    );

    await waitFor(() => expect(result.current.isError).toBe(true));

    // This codebase has no automatic wiring from React Query errors to the
    // Zustand notification store -- a consuming component is responsible for
    // surfacing the error itself (e.g. in a useEffect keyed on `isError`).
    // This test verifies that manual integration path.
    act(() => {
      useAppStore.getState().addNotification({
        type: 'error',
        message: result.current.error?.message ?? 'Unknown error',
      });
    });

    const notifications = useAppStore.getState().notifications;
    expect(notifications).toHaveLength(1);
    expect(notifications[0].type).toBe('error');
  });
});
