import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo from '@react-native-community/netinfo';
import { renderHook, waitFor, act } from '@testing-library/react-native';

import {
  generateMockCorridorData,
  parseCorridorAssets,
  useCorridorDetail,
} from '@hooks/useCorridorDetail';
import { CORRIDOR_DETAIL_CACHE_PREFIX } from '@app-types/corridor';

jest.mock('@services/api', () => ({
  apiClient: {
    get: jest.fn(),
  },
}));

import { apiClient } from '@services/api';

describe('useCorridorDetail', () => {
  beforeEach(async () => {
    jest.clearAllMocks();
    await AsyncStorage.clear();
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: true,
      isInternetReachable: true,
    });
  });

  it('loads corridor detail from API and caches result', async () => {
    const mockData = generateMockCorridorData('USDC-PHP');
    (apiClient.get as jest.Mock).mockResolvedValue(mockData);

    const { result } = renderHook(() =>
      useCorridorDetail({ corridorId: 'USDC-PHP' }),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.data?.corridor.id).toBe('USDC-PHP');
    expect(result.current.dataSource).toBe('live');
    expect(result.current.warning).toBeNull();
  });

  it('falls back to cached data when API fails', async () => {
    const mockData = generateMockCorridorData('USDC-PHP');
    await AsyncStorage.setItem(
      `${CORRIDOR_DETAIL_CACHE_PREFIX}USDC-PHP`,
      JSON.stringify(mockData),
    );
    (apiClient.get as jest.Mock).mockRejectedValue(new Error('Network error'));

    const { result } = renderHook(() =>
      useCorridorDetail({ corridorId: 'USDC-PHP' }),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('cache');
    expect(result.current.warning).toContain('saved corridor data');
  });

  it('uses mock data with clear labeling when offline and no cache exists', async () => {
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: false,
      isInternetReachable: false,
    });

    const { result } = renderHook(() =>
      useCorridorDetail({ corridorId: 'USDC-NGN' }),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
    expect(result.current.warning).toBe(
      'Offline — no saved data available. Showing sample data.',
    );
  });

  it('uses mock data with warning when online API fails and no cache exists', async () => {
    (apiClient.get as jest.Mock).mockRejectedValue(new Error('404'));

    const { result } = renderHook(() =>
      useCorridorDetail({ corridorId: 'USDC-PHP' }),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
    expect(result.current.warning).toBe('Live data unavailable. Showing sample data.');
  });

  it('sets error when corridor id is missing', async () => {
    const { result } = renderHook(() => useCorridorDetail({ corridorId: '' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBe('Corridor ID is required');
  });

  it('refetches corridor detail on demand', async () => {
    const mockData = generateMockCorridorData('USDC-PHP');
    (apiClient.get as jest.Mock).mockResolvedValue(mockData);

    const { result } = renderHook(() =>
      useCorridorDetail({ corridorId: 'USDC-PHP' }),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    await act(async () => {
      await result.current.refetch();
    });

    expect(apiClient.get).toHaveBeenCalledTimes(2);
  });

  it('uses offline cache when device is offline', async () => {
    const mockData = generateMockCorridorData('USDC-JPY');
    await AsyncStorage.setItem(
      `${CORRIDOR_DETAIL_CACHE_PREFIX}USDC-JPY`,
      JSON.stringify(mockData),
    );
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: false,
      isInternetReachable: false,
    });

    const { result } = renderHook(() =>
      useCorridorDetail({ corridorId: 'USDC-JPY' }),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('cache');
    expect(result.current.warning).toBe('Offline — showing saved corridor data.');
  });

  it('handles corrupted cache reads gracefully', async () => {
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: false,
      isInternetReachable: false,
    });
    (AsyncStorage.getItem as jest.Mock).mockRejectedValueOnce(
      new Error('Storage read failed'),
    );

    const { result } = renderHook(() =>
      useCorridorDetail({ corridorId: 'USDC-PHP' }),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
  });
});

describe('parseCorridorAssets', () => {
  it('parses backend corridor keys', () => {
    expect(parseCorridorAssets('USDC:GISSUER->PHP:GISSUER')).toEqual([
      'USDC',
      'PHP',
    ]);
  });
});
