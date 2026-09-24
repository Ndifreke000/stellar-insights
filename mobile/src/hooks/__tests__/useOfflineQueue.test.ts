import { processOfflineQueue } from '../useOfflineQueue';
import { apiClient } from '@services/api';
import { storageUtils } from '@services/storage';
import { useAppStore } from '@store/appStore';

jest.mock('@services/api', () => ({
  apiClient: {
    post: jest.fn(),
    put: jest.fn(),
    delete: jest.fn(),
  },
}));

jest.mock('@services/storage', () => ({
  storageUtils: {
    setItem: jest.fn(),
    getItem: jest.fn(),
    removeItem: jest.fn(),
  },
}));

jest.mock('@store/appStore', () => ({
  useAppStore: {
    getState: jest.fn(),
  },
}));

const mockedApiClient = apiClient as jest.Mocked<typeof apiClient>;
const mockedStorageUtils = storageUtils as jest.Mocked<typeof storageUtils>;
const mockedUseAppStore = useAppStore as unknown as { getState: jest.Mock };

describe('processOfflineQueue', () => {
  const setSyncStatus = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    mockedUseAppStore.getState.mockReturnValue({ isOnline: true, setSyncStatus });
  });

  it('keeps queued items when offline', async () => {
    const items = [
      {
        id: '1',
        method: 'POST' as const,
        url: '/payments',
        retryCount: 0,
        status: 'pending' as const,
        createdAt: '2026-01-01T00:00:00.000Z',
        updatedAt: '2026-01-01T00:00:00.000Z',
      },
    ];
    mockedUseAppStore.getState.mockReturnValue({ isOnline: false, setSyncStatus });
    mockedStorageUtils.getItem.mockReturnValue(JSON.stringify(items));

    await expect(processOfflineQueue()).resolves.toEqual(items);
    expect(mockedApiClient.post).not.toHaveBeenCalled();
  });

  it('processes queued requests when online', async () => {
    mockedStorageUtils.getItem.mockReturnValue(
      JSON.stringify([
        {
          id: '1',
          method: 'POST',
          url: '/payments',
          payload: { amount: 10 },
          retryCount: 0,
          status: 'pending',
          createdAt: '2026-01-01T00:00:00.000Z',
          updatedAt: '2026-01-01T00:00:00.000Z',
        },
      ])
    );
    mockedApiClient.post.mockResolvedValue(undefined);

    await expect(processOfflineQueue()).resolves.toEqual([]);

    expect(mockedApiClient.post).toHaveBeenCalledWith('/payments', { amount: 10 });
    expect(mockedStorageUtils.setItem).toHaveBeenCalledWith('offline-queue:v1', '[]');
    expect(setSyncStatus).toHaveBeenCalledWith(true);
    expect(setSyncStatus).toHaveBeenCalledWith(false);
  });

  it('retains failed requests with retry metadata', async () => {
    mockedStorageUtils.getItem.mockReturnValue(
      JSON.stringify([
        {
          id: '1',
          method: 'PUT',
          url: '/payments/1',
          payload: { amount: 20 },
          retryCount: 0,
          status: 'pending',
          createdAt: '2026-01-01T00:00:00.000Z',
          updatedAt: '2026-01-01T00:00:00.000Z',
        },
      ])
    );
    mockedApiClient.put.mockRejectedValue(new Error('Network unavailable'));

    const result = await processOfflineQueue();

    expect(result).toHaveLength(1);
    expect(result[0]).toMatchObject({
      id: '1',
      retryCount: 1,
      status: 'failed',
      lastError: 'Network unavailable',
    });
  });

  it('does not automatically retry failed requests until they are reset', async () => {
    const failedItem = {
      id: '1',
      method: 'POST' as const,
      url: '/payments',
      payload: { amount: 10 },
      retryCount: 1,
      status: 'failed' as const,
      createdAt: '2026-01-01T00:00:00.000Z',
      updatedAt: '2026-01-01T00:00:00.000Z',
      lastError: 'Network unavailable',
    };
    mockedStorageUtils.getItem.mockReturnValue(JSON.stringify([failedItem]));

    await expect(processOfflineQueue()).resolves.toEqual([failedItem]);

    expect(mockedApiClient.post).not.toHaveBeenCalled();
    expect(mockedStorageUtils.setItem).toHaveBeenCalledWith(
      'offline-queue:v1',
      JSON.stringify([failedItem])
    );
  });

  it('keeps max retry failures in the queue for manual review', async () => {
    mockedStorageUtils.getItem.mockReturnValue(
      JSON.stringify([
        {
          id: '1',
          method: 'DELETE',
          url: '/payments/1',
          retryCount: 4,
          status: 'pending',
          createdAt: '2026-01-01T00:00:00.000Z',
          updatedAt: '2026-01-01T00:00:00.000Z',
        },
      ])
    );
    mockedApiClient.delete.mockRejectedValue(new Error('Server unavailable'));

    const result = await processOfflineQueue();

    expect(result).toHaveLength(1);
    expect(result[0]).toMatchObject({
      id: '1',
      retryCount: 5,
      status: 'failed',
    });
    expect(result[0].lastError).toContain('Maximum retry count reached');
  });
});
