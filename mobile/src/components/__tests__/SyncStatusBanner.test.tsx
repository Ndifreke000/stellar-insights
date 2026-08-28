import React from 'react';
import renderer, { act } from 'react-test-renderer';
import { Text } from 'react-native';
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

  it('renders nothing when online with no pending or failed items', () => {
    const tree = renderer.create(<SyncStatusBanner />);
    expect(tree.toJSON()).toBeNull();
  });

  it('shows syncing indicator when isSyncing is true', () => {
    mockedUseAppStore.mockReturnValue({
      isSyncing: true,
      isOnline: true,
    } as ReturnType<typeof useAppStore>);

    const tree = renderer.create(<SyncStatusBanner />);
    const texts = tree.root.findAllByType(Text).map(n => n.props.children);
    expect(texts).toContain('Syncing…');
  });

  it('shows syncing indicator when queue isProcessing', () => {
    mockedUseOfflineQueue.mockReturnValue({
      ...defaultQueueState,
      isProcessing: true,
    });

    const tree = renderer.create(<SyncStatusBanner />);
    const texts = tree.root.findAllByType(Text).map(n => n.props.children);
    expect(texts).toContain('Syncing…');
  });

  it('shows failed banner when failed items exist', () => {
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

    const tree = renderer.create(<SyncStatusBanner />);
    const texts = tree.root.findAllByType(Text).map(n => n.props.children);
    expect(texts.some(t => String(t).includes('1 sync'))).toBe(true);
    expect(texts.some(t => String(t).includes('failed'))).toBe(true);
  });

  it('calls retryFailed when Retry is pressed', () => {
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

    const tree = renderer.create(<SyncStatusBanner />);
    const retryBtn = tree.root.findByProps({
      accessibilityLabel: 'Retry failed sync requests',
    });

    act(() => {
      retryBtn.props.onPress();
    });

    expect(retryFailed).toHaveBeenCalled();
  });

  it('calls onRetry prop when Retry is pressed', () => {
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

    const tree = renderer.create(<SyncStatusBanner onRetry={onRetry} />);
    const retryBtn = tree.root.findByProps({
      accessibilityLabel: 'Retry failed sync requests',
    });

    act(() => {
      retryBtn.props.onPress();
    });

    expect(onRetry).toHaveBeenCalled();
  });

  it('shows pending banner when offline with queued items', () => {
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

    const tree = renderer.create(<SyncStatusBanner />);
    const texts = tree.root.findAllByType(Text).map(n => n.props.children);
    expect(texts.some(t => String(t).includes('1 update'))).toBe(true);
  });

  it('renders nothing when offline but queue is empty', () => {
    mockedUseAppStore.mockReturnValue({
      isSyncing: false,
      isOnline: false,
    } as ReturnType<typeof useAppStore>);

    const tree = renderer.create(<SyncStatusBanner />);
    expect(tree.toJSON()).toBeNull();
  });

  it('uses provided testID', () => {
    mockedUseAppStore.mockReturnValue({
      isSyncing: true,
      isOnline: true,
    } as ReturnType<typeof useAppStore>);

    const tree = renderer.create(<SyncStatusBanner testID="sync-banner" />);
    const el = tree.root.findByProps({ testID: 'sync-banner' });
    expect(el).toBeTruthy();
  });
});
