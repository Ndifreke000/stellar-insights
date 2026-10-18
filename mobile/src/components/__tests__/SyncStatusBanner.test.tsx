import React from 'react';
import { render, fireEvent } from '@testing-library/react-native';
import { SyncStatusBanner } from '../SyncStatusBanner';
import { useAppStore } from '@store/appStore';
import { useOfflineQueue } from '@hooks/useOfflineQueue';

jest.mock('@store/appStore', () => ({
  useAppStore: jest.fn(),
}));

jest.mock('@hooks/useOfflineQueue', () => ({
  useOfflineQueue: jest.fn(),
}));

const mockedUseAppStore = useAppStore as jest.MockedFunction<typeof useAppStore>;
const mockedUseOfflineQueue = useOfflineQueue as jest.MockedFunction<typeof useOfflineQueue>;

const defaultQueueState = {
  items: [],
  isProcessing: false,
  error: undefined,
  enqueue: jest.fn(),
  remove: jest.fn(),
  clear: jest.fn(),
  retryFailed: jest.fn(),
  processQueue: jest.fn(),
};

describe('SyncStatusBanner', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockedUseAppStore.mockReturnValue({
      isSyncing: false,
      isOnline: true,
    } as ReturnType<typeof useAppStore>);
    mockedUseOfflineQueue.mockReturnValue(defaultQueueState);
  });

  it('renders nothing when online with no pending or failed items', async () => {
    const { toJSON } = await render(<SyncStatusBanner />);
    expect(toJSON()).toBeNull();
  });

  it('shows syncing indicator when isSyncing is true', async () => {
    mockedUseAppStore.mockReturnValue({
      isSyncing: true,
      isOnline: true,
    } as ReturnType<typeof useAppStore>);

    const { getByText } = await render(<SyncStatusBanner />);
    expect(getByText('Syncing…')).toBeTruthy();
  });

  it('shows syncing indicator when queue isProcessing', async () => {
    mockedUseOfflineQueue.mockReturnValue({
      ...defaultQueueState,
      isProcessing: true,
    });

    const { getByText } = await render(<SyncStatusBanner />);
    expect(getByText('Syncing…')).toBeTruthy();
  });

  it('shows failed banner when failed items exist', async () => {
    mockedUseOfflineQueue.mockReturnValue({
      ...defaultQueueState,
      items: [
        {
          id: '1',
          method: 'POST',
          url: '/payments',
          retryCount: 3,
          status: 'failed',
          createdAt: '2026-01-01T00:00:00.000Z',
          updatedAt: '2026-01-01T00:00:00.000Z',
          lastError: 'Network error',
        },
      ],
    });

    const { getByText } = await render(<SyncStatusBanner />);
    expect(getByText(/1 sync/)).toBeTruthy();
    expect(getByText(/failed/)).toBeTruthy();
  });

  it('calls retryFailed when Retry is pressed', async () => {
    const retryFailed = jest.fn().mockResolvedValue(undefined);
    mockedUseOfflineQueue.mockReturnValue({
      ...defaultQueueState,
      retryFailed,
      items: [
        {
          id: '1',
          method: 'POST',
          url: '/payments',
          retryCount: 1,
          status: 'failed',
          createdAt: '2026-01-01T00:00:00.000Z',
          updatedAt: '2026-01-01T00:00:00.000Z',
        },
      ],
    });

    const { getByLabelText } = await render(<SyncStatusBanner />);
    const retryBtn = getByLabelText('Retry failed sync requests');

    await fireEvent.press(retryBtn);

    expect(retryFailed).toHaveBeenCalled();
  });

  it('calls onRetry prop when Retry is pressed', async () => {
    const onRetry = jest.fn();
    const retryFailed = jest.fn().mockResolvedValue(undefined);
    mockedUseOfflineQueue.mockReturnValue({
      ...defaultQueueState,
      retryFailed,
      items: [
        {
          id: '1',
          method: 'DELETE',
          url: '/payments/1',
          retryCount: 2,
          status: 'failed',
          createdAt: '2026-01-01T00:00:00.000Z',
          updatedAt: '2026-01-01T00:00:00.000Z',
        },
      ],
    });

    const { getByLabelText } = await render(<SyncStatusBanner onRetry={onRetry} />);
    const retryBtn = getByLabelText('Retry failed sync requests');

    await fireEvent.press(retryBtn);

    expect(onRetry).toHaveBeenCalled();
  });

  it('shows pending banner when offline with queued items', async () => {
    mockedUseAppStore.mockReturnValue({
      isSyncing: false,
      isOnline: false,
    } as ReturnType<typeof useAppStore>);
    mockedUseOfflineQueue.mockReturnValue({
      ...defaultQueueState,
      items: [
        {
          id: '2',
          method: 'PUT',
          url: '/settings',
          retryCount: 0,
          status: 'pending',
          createdAt: '2026-01-01T00:00:00.000Z',
          updatedAt: '2026-01-01T00:00:00.000Z',
        },
      ],
    });

    const { getByText } = await render(<SyncStatusBanner />);
    expect(getByText(/1 update/)).toBeTruthy();
  });

  it('renders nothing when offline but queue is empty', async () => {
    mockedUseAppStore.mockReturnValue({
      isSyncing: false,
      isOnline: false,
    } as ReturnType<typeof useAppStore>);

    const { toJSON } = await render(<SyncStatusBanner />);
    expect(toJSON()).toBeNull();
  });

  it('uses provided testID', async () => {
    mockedUseAppStore.mockReturnValue({
      isSyncing: true,
      isOnline: true,
    } as ReturnType<typeof useAppStore>);

    const { getByTestId } = await render(<SyncStatusBanner testID="sync-banner" />);
    expect(getByTestId('sync-banner')).toBeTruthy();
  });
});
