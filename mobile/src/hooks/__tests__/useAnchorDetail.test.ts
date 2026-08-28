import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo from '@react-native-community/netinfo';
import { renderHook, waitFor, act } from '@testing-library/react-native';

import {
  generateMockAnchorDetail,
  normalizeAnchorDetailResponse,
  useAnchorDetail,
} from '@hooks/useAnchorDetail';
import { ANCHOR_DETAIL_CACHE_PREFIX } from '@app-types/anchor';

jest.mock('@services/api', () => ({
  apiClient: {
    get: jest.fn(),
  },
}));

import { apiClient } from '@services/api';

describe('useAnchorDetail', () => {
  beforeEach(async () => {
    jest.clearAllMocks();
    await AsyncStorage.clear();
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: true,
      isInternetReachable: true,
    });
  });

  it('loads anchor detail from API and caches result', async () => {
    const mockData = generateMockAnchorDetail('anchor-1');
    (apiClient.get as jest.Mock).mockResolvedValue(mockData);

    const { result } = renderHook(() => useAnchorDetail({ anchorId: 'anchor-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.data?.anchor.id).toBe('anchor-1');
    expect(result.current.dataSource).toBe('live');
    expect(result.current.warning).toBeNull();
  });

  it('falls back to cached data when API fails', async () => {
    const mockData = generateMockAnchorDetail('anchor-1');
    await AsyncStorage.setItem(
      `${ANCHOR_DETAIL_CACHE_PREFIX}anchor-1`,
      JSON.stringify(mockData),
    );
    (apiClient.get as jest.Mock).mockRejectedValue(new Error('Network error'));

    const { result } = renderHook(() => useAnchorDetail({ anchorId: 'anchor-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('cache');
    expect(result.current.warning).toContain('saved anchor data');
  });

  it('uses mock data with clear labeling when offline and no cache exists', async () => {
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: false,
      isInternetReachable: false,
    });

    const { result } = renderHook(() => useAnchorDetail({ anchorId: 'anchor-2' }));

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

    const { result } = renderHook(() => useAnchorDetail({ anchorId: 'anchor-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
    expect(result.current.warning).toBe('Live data unavailable. Showing sample data.');
  });

  it('sets error when anchor id is missing', async () => {
    const { result } = renderHook(() => useAnchorDetail({ anchorId: '' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBe('Anchor ID is required');
  });

  it('refetches anchor detail on demand', async () => {
    const mockData = generateMockAnchorDetail('anchor-1');
    (apiClient.get as jest.Mock).mockResolvedValue(mockData);

    const { result } = renderHook(() => useAnchorDetail({ anchorId: 'anchor-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    await act(async () => {
      await result.current.refetch();
    });

    expect(apiClient.get).toHaveBeenCalledTimes(2);
  });

  it('uses offline cache when device is offline', async () => {
    const mockData = generateMockAnchorDetail('anchor-3');
    await AsyncStorage.setItem(
      `${ANCHOR_DETAIL_CACHE_PREFIX}anchor-3`,
      JSON.stringify(mockData),
    );
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: false,
      isInternetReachable: false,
    });

    const { result } = renderHook(() => useAnchorDetail({ anchorId: 'anchor-3' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('cache');
    expect(result.current.warning).toBe('Offline — showing saved anchor data.');
  });

  it('handles corrupted cache reads gracefully', async () => {
    (NetInfo.fetch as jest.Mock).mockResolvedValue({
      isConnected: false,
      isInternetReachable: false,
    });
    (AsyncStorage.getItem as jest.Mock).mockRejectedValueOnce(
      new Error('Storage read failed'),
    );

    const { result } = renderHook(() => useAnchorDetail({ anchorId: 'anchor-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.dataSource).toBe('mock');
  });

  it('treats device as offline when NetInfo fetch fails', async () => {
    (NetInfo.fetch as jest.Mock).mockRejectedValue(new Error('NetInfo unavailable'));

    const { result } = renderHook(() => useAnchorDetail({ anchorId: 'anchor-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.isOffline).toBe(true);
    expect(result.current.dataSource).toBe('mock');
  });

  it('fetches by stellar account when anchor id is a G-address', async () => {
    const stellarAccount =
      'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN';
    const mockData = generateMockAnchorDetail(stellarAccount);
    (apiClient.get as jest.Mock).mockResolvedValue(mockData);

    const { result } = renderHook(() =>
      useAnchorDetail({ anchorId: stellarAccount }),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(apiClient.get).toHaveBeenCalledWith(
      `/anchors/account/${encodeURIComponent(stellarAccount)}`,
    );
  });
});

describe('normalizeAnchorDetailResponse', () => {
  it('returns issued_assets when already normalized', () => {
    const issued_assets = [
      {
        asset_code: 'USDC',
        issuer: 'GISSUER',
        volume_24h_usd: 100,
        success_rate: 99,
        failure_rate: 1,
        total_transactions: 1,
      },
    ];

    const normalized = normalizeAnchorDetailResponse(
      {
        anchor: { id: 'a1', name: 'A', stellar_account: 'G1' },
        issued_assets,
        reliability_history: [{ timestamp: '2026-05-01', score: 90 }],
      },
      'a1',
    );

    expect(normalized.issued_assets).toEqual(issued_assets);
  });

  it('maps backend assets and metrics_history fields', () => {
    const normalized = normalizeAnchorDetailResponse(
      {
        anchor: {
          id: 'anchor-1',
          name: 'Test Anchor',
          stellar_account: 'GSHORTACCOUNT',
          reliability_score: 88,
          total_transactions: 100,
          successful_transactions: 90,
          failed_transactions: 10,
          status: 'green',
        },
        assets: [
          {
            asset_code: 'USDC',
            asset_issuer: 'GISSUER',
          },
        ],
        metrics_history: [
          {
            timestamp: '2026-05-01T12:00:00Z',
            reliability_score: 88,
          },
        ],
      },
      'anchor-1',
    );

    expect(normalized.issued_assets).toHaveLength(1);
    expect(normalized.issued_assets[0].asset_code).toBe('USDC');
    expect(normalized.reliability_history).toHaveLength(1);
    expect(normalized.reliability_history[0].score).toBe(88);
    expect(normalized.anchor.asset_coverage).toBe(1);
  });

  it('uses explicit failure_rate and computes from transactions when absent', () => {
    const withExplicit = normalizeAnchorDetailResponse(
      {
        anchor: {
          id: 'a1',
          failure_rate: 12.5,
          total_transactions: 0,
          failed_transactions: 0,
        },
      },
      'a1',
    );
    expect(withExplicit.anchor.failure_rate).toBe(12.5);

    const computed = normalizeAnchorDetailResponse(
      {
        anchor: {
          id: 'a2',
          total_transactions: 200,
          failed_transactions: 20,
          successful_transactions: 180,
        },
      },
      'a2',
    );
    expect(computed.anchor.failure_rate).toBe(10);
  });

  it('returns empty reliability history when metrics are missing', () => {
    const normalized = normalizeAnchorDetailResponse(
      { anchor: { id: 'a1' } },
      'a1',
    );
    expect(normalized.reliability_history).toEqual([]);
  });
});

describe('generateMockAnchorDetail', () => {
  it('returns stable mock structure', () => {
    const data = generateMockAnchorDetail('anchor-1');
    expect(data.anchor.name).toBe('Simulated Anchor Inc.');
    expect(data.issued_assets.length).toBeGreaterThan(0);
    expect(data.reliability_history.length).toBe(30);
  });

  it('uses provided stellar account for mock G-address ids', () => {
    const stellarAccount =
      'GBBD47IF6LWK7P7MDEVSCWR7DPUXV3NAM7KNR4WTXV7X5FDT5O6ADFOY';
    const data = generateMockAnchorDetail(stellarAccount);
    expect(data.anchor.stellar_account).toBe(stellarAccount);
    expect(data.anchor.id).toBe(stellarAccount);
  });
});
