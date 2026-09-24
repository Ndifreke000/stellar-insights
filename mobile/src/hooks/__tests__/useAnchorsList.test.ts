import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo from '@react-native-community/netinfo';
import { renderHook, waitFor, act } from '@testing-library/react-native';

import {
  ANCHORS_LIST_CACHE_KEY,
  generateMockAnchorsList,
  normalizeAnchorsResponse,
  useAnchorsList,
} from '@hooks/useAnchorsList';

jest.mock('@services/api', () => ({
  apiClient: {
    get: jest.fn(),
  },
}));

import { apiClient } from '@services/api';

describe('useAnchorsList', () => {
  beforeEach(async () => {
    jest.clearAllMocks();
    await AsyncStorage.clear();
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: true,
      isInternetReachable: true,
    });
  });

  it('loads anchors from API and caches result', async () => {
    const mockData = generateMockAnchorsList();
    (apiClient.get as jest.Mock).mockResolvedValue(mockData);

    const { result } = await renderHook(() => useAnchorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.anchors).toHaveLength(3);
    expect(result.current.dataSource).toBe('live');
    expect(result.current.warning).toBeNull();
  });

  it('normalizes paginated API responses from the backend', async () => {
    const mockData = generateMockAnchorsList();
    (apiClient.get as jest.Mock).mockResolvedValue({
      data: mockData.anchors,
      pagination: { total: mockData.total },
    });

    const { result } = await renderHook(() => useAnchorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.anchors).toHaveLength(3);
    expect(result.current.total).toBe(3);
    expect(result.current.dataSource).toBe('live');
  });

  it('falls back to cached data when API fails', async () => {
    const mockData = generateMockAnchorsList();
    await AsyncStorage.setItem(ANCHORS_LIST_CACHE_KEY, JSON.stringify(mockData));
    (apiClient.get as jest.Mock).mockRejectedValue(new Error('Network error'));

    const { result } = await renderHook(() => useAnchorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('cache');
    expect(result.current.warning).toContain('saved anchors');
  });

  it('uses mock data with clear labeling when offline and no cache exists', async () => {
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: false,
      isInternetReachable: false,
    });

    const { result } = await renderHook(() => useAnchorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
    expect(result.current.warning).toBe(
      'Offline — no saved data available. Showing sample anchors.',
    );
  });

  it('uses mock data with warning when online API fails and no cache exists', async () => {
    (apiClient.get as jest.Mock).mockRejectedValue(new Error('404'));

    const { result } = await renderHook(() => useAnchorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
    expect(result.current.warning).toBe('Live data unavailable. Showing sample anchors.');
  });

  it('refetches anchors list on demand', async () => {
    const mockData = generateMockAnchorsList();
    (apiClient.get as jest.Mock).mockResolvedValue(mockData);

    const { result } = await renderHook(() => useAnchorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    await act(async () => {
      await result.current.refetch();
    });

    expect(apiClient.get).toHaveBeenCalledTimes(2);
  });

  it('uses offline cache when device is offline', async () => {
    const mockData = generateMockAnchorsList();
    await AsyncStorage.setItem(ANCHORS_LIST_CACHE_KEY, JSON.stringify(mockData));
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: false,
      isInternetReachable: false,
    });

    const { result } = await renderHook(() => useAnchorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('cache');
    expect(result.current.warning).toBe('Offline — showing saved anchors.');
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

    const { result } = await renderHook(() => useAnchorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
  });

  it('handles NetInfo fetch failures as offline', async () => {
    (NetInfo.fetch as jest.Mock).mockRejectedValue(new Error('NetInfo unavailable'));

    const { result } = await renderHook(() => useAnchorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.isOffline).toBe(true);
    expect(result.current.dataSource).toBe('mock');
  });
});

describe('generateMockAnchorsList', () => {
  it('returns sample anchors with total count', () => {
    const data = generateMockAnchorsList();
    expect(data.anchors.length).toBeGreaterThan(0);
    expect(data.total).toBe(data.anchors.length);
  });
});

describe('normalizeAnchorsResponse', () => {
  it('accepts legacy anchors/total responses', () => {
    const mockData = generateMockAnchorsList();
    expect(normalizeAnchorsResponse(mockData)).toEqual(mockData);
  });

  it('accepts paginated data/pagination responses', () => {
    const mockData = generateMockAnchorsList();
    expect(
      normalizeAnchorsResponse({
        data: mockData.anchors,
        pagination: { total: mockData.total },
      }),
    ).toEqual(mockData);
  });

  it('throws for invalid responses', () => {
    expect(() => normalizeAnchorsResponse({})).toThrow('Invalid anchors response');
  });
});
