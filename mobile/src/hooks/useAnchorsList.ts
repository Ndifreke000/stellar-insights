import { useCallback } from 'react';

import { apiClient } from '@services/api';
import { CACHE_KEYS } from '@config/constants';
import {
  CursorPage,
  usePaginatedList,
  UsePaginatedListReturn,
} from './usePaginatedList';

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

interface AnchorsCursorResponse {
  data: AnchorListItem[];
  next_cursor: string | null;
  total: number;
}

function normalizeAnchorsPage(raw: unknown): CursorPage<AnchorListItem> {
  const r = raw as Partial<AnchorsCursorResponse>;
  if (Array.isArray(r.data)) {
    return {
      items: r.data,
      next_cursor: r.next_cursor ?? null,
      total: r.total ?? r.data.length,
    };
  }

  // Legacy: plain array
  if (Array.isArray(raw)) {
    return { items: raw as AnchorListItem[], next_cursor: null, total: (raw as AnchorListItem[]).length };
  }

  throw new Error('Invalid anchors response');
}

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

function mockAnchorsPage(): CursorPage<AnchorListItem> {
  return { items: MOCK_ANCHORS, next_cursor: null, total: MOCK_ANCHORS.length };
}

export type UseAnchorsListReturn = UsePaginatedListReturn<AnchorListItem>;

export function useAnchorsList(): UseAnchorsListReturn {
  const fetchPage = useCallback(async (cursor: string | null) => {
    const params = cursor ? { cursor, limit: 20 } : { limit: 20 };
    const response = await apiClient.get<unknown>('/anchors', { params });
    return normalizeAnchorsPage(response);
  }, []);

  return usePaginatedList<AnchorListItem>({
    cacheKey: CACHE_KEYS.ANCHORS,
    fetchPage,
    mockData: mockAnchorsPage,
  });
}
