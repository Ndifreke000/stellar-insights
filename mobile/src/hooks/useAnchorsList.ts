import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo from '@react-native-community/netinfo';
import { useCallback, useEffect, useState } from 'react';

import { apiClient } from '@services/api';
import { CACHE_KEYS } from '@config/constants';

export type AnchorDataSource = 'live' | 'cache' | 'mock';

export interface AnchorListItem {
  id: string;
  name: string;
  stellar_account: string;
  reliability_score: number;
  asset_coverage: number;
  failure_rate: number;
  total_transactions: number;
  successful_transactions: number;
  failed_transactions: number;
  status: string;
}

export interface AnchorsListData {
  anchors: AnchorListItem[];
  total: number;
}

interface PaginatedAnchorsResponse {
  data: AnchorListItem[];
  pagination: {
    total: number;
  };
}

export function normalizeAnchorsResponse(raw: unknown): AnchorsListData {
  const response = raw as Partial<AnchorsListData & PaginatedAnchorsResponse>;

  if (Array.isArray(response.anchors)) {
    return {
      anchors: response.anchors,
      total: response.total ?? response.anchors.length,
    };
  }

  if (Array.isArray(response.data)) {
    return {
      anchors: response.data,
      total: response.pagination?.total ?? response.data.length,
    };
  }

  throw new Error('Invalid anchors response');
}

export interface UseAnchorsListReturn {
  anchors: AnchorListItem[];
  total: number;
  loading: boolean;
  error: string | null;
  warning: string | null;
  isOffline: boolean;
  dataSource: AnchorDataSource | null;
  isFromCache: boolean;
  refetch: () => Promise<void>;
}

export const ANCHORS_LIST_CACHE_KEY = CACHE_KEYS.ANCHORS;

const MOCK_ANCHORS: AnchorListItem[] = [
  {
    id: 'anchor-1',
    name: 'MoneyGram Access',
    stellar_account: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    reliability_score: 99.2,
    asset_coverage: 5,
    failure_rate: 0.8,
    total_transactions: 12450,
    successful_transactions: 12350,
    failed_transactions: 100,
    status: 'green',
  },
  {
    id: 'anchor-2',
    name: 'AnchorUSD',
    stellar_account: 'GBBD47IF6LWK7P7MDEVSCWR7DPUXV3NAM7KNR4WTXV7X5FDT5O6ADFOY',
    reliability_score: 91.5,
    asset_coverage: 3,
    failure_rate: 8.5,
    total_transactions: 8320,
    successful_transactions: 7610,
    failed_transactions: 710,
    status: 'yellow',
  },
  {
    id: 'anchor-3',
    name: 'Demo Anchor',
    stellar_account: 'GCKFBEIYTKPGAQQLRGSTNATTJHUUOWD63AANJPLUYFXXQWSK3PXMKYC7',
    reliability_score: 72.4,
    asset_coverage: 2,
    failure_rate: 27.6,
    total_transactions: 2100,
    successful_transactions: 1520,
    failed_transactions: 580,
    status: 'red',
  },
];

export function generateMockAnchorsList(): AnchorsListData {
  return {
    anchors: MOCK_ANCHORS,
    total: MOCK_ANCHORS.length,
  };
}

async function readCachedAnchors(): Promise<AnchorsListData | null> {
  try {
    const cached = await AsyncStorage.getItem(ANCHORS_LIST_CACHE_KEY);
    return cached ? (JSON.parse(cached) as AnchorsListData) : null;
  } catch {
    return null;
  }
}

async function writeCachedAnchors(data: AnchorsListData): Promise<void> {
  try {
    await AsyncStorage.setItem(ANCHORS_LIST_CACHE_KEY, JSON.stringify(data));
  } catch {
    // Cache writes are best-effort for offline support.
  }
}

async function fetchAnchorsList(): Promise<AnchorsListData> {
  const response = await apiClient.get<unknown>('/anchors');
  return normalizeAnchorsResponse(response);
}

function applyFallbackResult(
  setAnchors: (anchors: AnchorListItem[]) => void,
  setTotal: (total: number) => void,
  setDataSource: (source: AnchorDataSource) => void,
  setWarning: (warning: string) => void,
  dataSource: Extract<AnchorDataSource, 'cache' | 'mock'>,
  warning: string,
  cachedData?: AnchorsListData | null,
): void {
  const data =
    dataSource === 'cache' && cachedData
      ? cachedData
      : generateMockAnchorsList();

  setAnchors(data.anchors);
  setTotal(data.total);
  setDataSource(dataSource);
  setWarning(warning);
}

export function useAnchorsList(): UseAnchorsListReturn {
  const [anchors, setAnchors] = useState<AnchorListItem[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [isOffline, setIsOffline] = useState(false);
  const [dataSource, setDataSource] = useState<AnchorDataSource | null>(null);

  const loadAnchorsList = useCallback(async () => {
    setLoading(true);
    setError(null);
    setWarning(null);
    setDataSource(null);

    let offline = false;
    try {
      const networkState = await NetInfo.fetch();
      offline = !(
        networkState.isConnected &&
        networkState.isInternetReachable !== false
      );
    } catch {
      offline = true;
    }
    setIsOffline(offline);

    try {
      if (offline) {
        const cached = await readCachedAnchors();
        if (cached) {
          setAnchors(cached.anchors);
          setTotal(cached.total);
          setDataSource('cache');
          setWarning('Offline — showing saved anchors.');
          return;
        }

        applyFallbackResult(
          setAnchors,
          setTotal,
          setDataSource,
          setWarning,
          'mock',
          'Offline — no saved data available. Showing sample anchors.',
        );
        return;
      }

      try {
        const result = await fetchAnchorsList();
        setAnchors(result.anchors);
        setTotal(result.total);
        setDataSource('live');
        await writeCachedAnchors(result);
        return;
      } catch {
        const cached = await readCachedAnchors();
        if (cached) {
          applyFallbackResult(
            setAnchors,
            setTotal,
            setDataSource,
            setWarning,
            'cache',
            'Live data unavailable. Showing saved anchors.',
            cached,
          );
          return;
        }

        applyFallbackResult(
          setAnchors,
          setTotal,
          setDataSource,
          setWarning,
          'mock',
          'Live data unavailable. Showing sample anchors.',
        );
      }
    } catch {
      setError('Failed to load anchors');
      setAnchors([]);
      setTotal(0);
      setDataSource(null);
      setWarning(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadAnchorsList();
  }, [loadAnchorsList]);

  return {
    anchors,
    total,
    loading,
    error,
    warning,
    isOffline,
    dataSource,
    isFromCache: dataSource === 'cache',
    refetch: loadAnchorsList,
  };
}
