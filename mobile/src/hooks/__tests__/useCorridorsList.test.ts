import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo from '@react-native-community/netinfo';
import { renderHook, waitFor, act } from '@testing-library/react-native';

import {
  CORRIDORS_LIST_CACHE_KEY,
  generateMockCorridorsList,
  normalizeCorridorsResponse,
  useCorridorsList,
} from '@hooks/useCorridorsList';

jest.mock('@services/api', () => ({
  apiClient: {
    get: jest.fn(),
  },
}));

import { apiClient } from '@services/api';

describe('useCorridorsList', () => {
  beforeEach(async () => {
    jest.clearAllMocks();
    await AsyncStorage.clear();
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: true,
      isInternetReachable: true,
    });
  });

  it('loads corridors from API and caches result', async () => {
    const mockData = generateMockCorridorsList();
    (apiClient.get as jest.Mock).mockResolvedValue(mockData.corridors);

    const { result } = renderHook(() => useCorridorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.corridors).toHaveLength(3);
    expect(result.current.dataSource).toBe('live');
    expect(result.current.warning).toBeNull();
  });

  it('normalizes corridors/total responses from the backend', async () => {
    const mockData = generateMockCorridorsList();
    (apiClient.get as jest.Mock).mockResolvedValue(mockData);

    const { result } = renderHook(() => useCorridorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.corridors).toHaveLength(3);
    expect(result.current.total).toBe(3);
    expect(result.current.dataSource).toBe('live');
  });

  it('normalizes paginated API responses from the backend', async () => {
    const mockData = generateMockCorridorsList();
    (apiClient.get as jest.Mock).mockResolvedValue({
      data: mockData.corridors,
      pagination: { total: mockData.total },
    });

    const { result } = renderHook(() => useCorridorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.corridors).toHaveLength(3);
    expect(result.current.total).toBe(3);
    expect(result.current.dataSource).toBe('live');
  });

  it('falls back to cached data when API fails', async () => {
    const mockData = generateMockCorridorsList();
    await AsyncStorage.setItem(CORRIDORS_LIST_CACHE_KEY, JSON.stringify(mockData));
    (apiClient.get as jest.Mock).mockRejectedValue(new Error('Network error'));

    const { result } = renderHook(() => useCorridorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('cache');
    expect(result.current.warning).toContain('saved corridors');
  });

  it('uses mock data with clear labeling when offline and no cache exists', async () => {
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: false,
      isInternetReachable: false,
    });

    const { result } = renderHook(() => useCorridorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
    expect(result.current.warning).toBe(
      'Offline — no saved data available. Showing sample corridors.',
    );
  });

  it('uses mock data with warning when online API fails and no cache exists', async () => {
    (apiClient.get as jest.Mock).mockRejectedValue(new Error('404'));

    const { result } = renderHook(() => useCorridorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
    expect(result.current.warning).toBe('Live data unavailable. Showing sample corridors.');
  });

  it('refetches corridors list on demand', async () => {
    const mockData = generateMockCorridorsList();
    (apiClient.get as jest.Mock).mockResolvedValue(mockData.corridors);

    const { result } = renderHook(() => useCorridorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    await act(async () => {
      await result.current.refetch();
    });

    expect(apiClient.get).toHaveBeenCalledTimes(2);
  });

  it('uses offline cache when device is offline', async () => {
    const mockData = generateMockCorridorsList();
    await AsyncStorage.setItem(CORRIDORS_LIST_CACHE_KEY, JSON.stringify(mockData));
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: false,
      isInternetReachable: false,
    });

    const { result } = renderHook(() => useCorridorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('cache');
    expect(result.current.warning).toBe('Offline — showing saved corridors.');
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

    const { result } = renderHook(() => useCorridorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
  });

  it('handles NetInfo fetch failures as offline', async () => {
    (NetInfo.fetch as jest.Mock).mockRejectedValue(new Error('NetInfo unavailable'));

    const { result } = renderHook(() => useCorridorsList());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.isOffline).toBe(true);
    expect(result.current.dataSource).toBe('mock');
  });
});

describe('generateMockCorridorsList', () => {
  it('returns sample corridors with total count', () => {
    const data = generateMockCorridorsList();
    expect(data.corridors.length).toBeGreaterThan(0);
    expect(data.total).toBe(data.corridors.length);
  });
});

describe('normalizeCorridorsResponse', () => {
  it('accepts plain array responses', () => {
    const mockData = generateMockCorridorsList();
    expect(normalizeCorridorsResponse(mockData.corridors)).toEqual(mockData);
  });

  it('accepts legacy corridors/total responses', () => {
    const mockData = generateMockCorridorsList();
    expect(normalizeCorridorsResponse(mockData)).toEqual(mockData);
  });

  it('accepts paginated data/pagination responses', () => {
    const mockData = generateMockCorridorsList();
    expect(
      normalizeCorridorsResponse({
        data: mockData.corridors,
        pagination: { total: mockData.total },
      }),
    ).toEqual(mockData);
  });

  it('throws for invalid responses', () => {
    expect(() => normalizeCorridorsResponse({})).toThrow('Invalid corridors response');
  });
});
