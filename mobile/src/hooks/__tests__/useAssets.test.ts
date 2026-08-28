import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo from '@react-native-community/netinfo';
import { renderHook, waitFor, act } from '@testing-library/react-native';

import {
  ASSETS_LIST_CACHE_KEY,
  generateMockAssetsList,
  normalizeAssetsResponse,
  useAssets,
} from '@hooks/useAssets';

jest.mock('@services/api', () => ({
  apiClient: {
    get: jest.fn(),
  },
}));

import { apiClient } from '@services/api';

describe('useAssets', () => {
  beforeEach(async () => {
    jest.clearAllMocks();
    await AsyncStorage.clear();
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: true,
      isInternetReachable: true,
    });
  });

  it('loads assets from API and caches result', async () => {
    const mockData = generateMockAssetsList();
    (apiClient.get as jest.Mock).mockResolvedValue(mockData);

    const { result } = renderHook(() => useAssets());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.assets).toHaveLength(3);
    expect(result.current.dataSource).toBe('live');
    expect(result.current.warning).toBeNull();
  });

  it('normalizes paginated API responses from the backend', async () => {
    const mockData = generateMockAssetsList();
    (apiClient.get as jest.Mock).mockResolvedValue({
      data: mockData.assets,
      pagination: { total: mockData.total },
    });

    const { result } = renderHook(() => useAssets());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.assets).toHaveLength(3);
    expect(result.current.total).toBe(3);
    expect(result.current.dataSource).toBe('live');
  });

  it('falls back to cached data when API fails', async () => {
    const mockData = generateMockAssetsList();
    await AsyncStorage.setItem(ASSETS_LIST_CACHE_KEY, JSON.stringify(mockData));
    (apiClient.get as jest.Mock).mockRejectedValue(new Error('Network error'));

    const { result } = renderHook(() => useAssets());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('cache');
    expect(result.current.warning).toContain('saved assets');
  });

  it('uses mock data with clear labeling when offline and no cache exists', async () => {
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: false,
      isInternetReachable: false,
    });

    const { result } = renderHook(() => useAssets());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
    expect(result.current.warning).toBe(
      'Offline — no saved data available. Showing sample assets.',
    );
  });

  it('uses mock data with warning when online API fails and no cache exists', async () => {
    (apiClient.get as jest.Mock).mockRejectedValue(new Error('404'));

    const { result } = renderHook(() => useAssets());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
    expect(result.current.warning).toBe('Live data unavailable. Showing sample assets.');
  });

  it('refetches assets on demand', async () => {
    const mockData = generateMockAssetsList();
    (apiClient.get as jest.Mock).mockResolvedValue(mockData);

    const { result } = renderHook(() => useAssets());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    await act(async () => {
      await result.current.refetch();
    });

    expect(apiClient.get).toHaveBeenCalledTimes(2);
  });

  it('uses offline cache when device is offline', async () => {
    const mockData = generateMockAssetsList();
    await AsyncStorage.setItem(ASSETS_LIST_CACHE_KEY, JSON.stringify(mockData));
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: false,
      isInternetReachable: false,
    });

    const { result } = renderHook(() => useAssets());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('cache');
    expect(result.current.warning).toBe('Offline — showing saved assets.');
    expect(result.current.isFromCache).toBe(true);
  });

  it('handles corrupted cache reads gracefully', async () => {
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: false,
      isInternetReachable: false,
    });
    (AsyncStorage.getItem as jest.Mock).mockRejectedValueOnce(
      new Error('Storage read failed'),
    );

    const { result } = renderHook(() => useAssets());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
  });

  it('handles NetInfo fetch failures as offline', async () => {
    (NetInfo.fetch as jest.Mock).mockRejectedValue(new Error('NetInfo unavailable'));

    const { result } = renderHook(() => useAssets());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.isOffline).toBe(true);
    expect(result.current.dataSource).toBe('mock');
  });

  it('writes live data to cache so it is available offline later', async () => {
    const mockData = generateMockAssetsList();
    (apiClient.get as jest.Mock).mockResolvedValue(mockData);

    const { result } = renderHook(() => useAssets());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const stored = await AsyncStorage.getItem(ASSETS_LIST_CACHE_KEY);
    expect(stored).not.toBeNull();
    const parsed = JSON.parse(stored!);
    expect(parsed.assets).toHaveLength(3);
  });
});

describe('generateMockAssetsList', () => {
  it('returns sample assets with total count', () => {
    const data = generateMockAssetsList();
    expect(data.assets.length).toBeGreaterThan(0);
    expect(data.total).toBe(data.assets.length);
  });
});

describe('normalizeAssetsResponse', () => {
  it('accepts plain array responses', () => {
    const mockData = generateMockAssetsList();
    expect(normalizeAssetsResponse(mockData.assets)).toEqual(mockData);
  });

  it('accepts legacy assets/total responses', () => {
    const mockData = generateMockAssetsList();
    expect(normalizeAssetsResponse(mockData)).toEqual(mockData);
  });

  it('accepts paginated data/pagination responses', () => {
    const mockData = generateMockAssetsList();
    expect(
      normalizeAssetsResponse({
        data: mockData.assets,
        pagination: { total: mockData.total },
      }),
    ).toEqual(mockData);
  });

  it('throws for invalid responses', () => {
    expect(() => normalizeAssetsResponse({})).toThrow('Invalid assets response');
  });
});
