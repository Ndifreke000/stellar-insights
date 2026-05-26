import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@services/api';
import { useAppStore } from '@store/appStore';

export type SearchResultType = 'anchor' | 'corridor';

export interface SearchResult {
  id: string;
  type: SearchResultType;
  title: string;
  subtitle: string;
  searchText: string;
}

interface AnchorRecord {
  id: string;
  name: string;
  stellar_account: string;
  status?: string;
}

interface ListResponse<T> {
  data: T[];
}

interface CorridorRecord {
  id: string;
  source_asset: string;
  destination_asset: string;
  success_rate?: number;
}

export function filterSearchResults(results: SearchResult[], query: string): SearchResult[] {
  const normalizedQuery = query.trim().toLowerCase();

  if (!normalizedQuery) {
    return [];
  }

  return results
    .filter(result => result.searchText.toLowerCase().includes(normalizedQuery))
    .slice(0, 30);
}

function buildSearchResults(anchors: AnchorRecord[], corridors: CorridorRecord[]): SearchResult[] {
  const anchorResults: SearchResult[] = anchors.map(anchor => ({
    id: anchor.id,
    type: 'anchor',
    title: anchor.name,
    subtitle: `${anchor.status ?? 'Anchor'} | ${anchor.stellar_account}`,
    searchText: `${anchor.name} ${anchor.stellar_account} ${anchor.status ?? ''}`,
  }));

  const corridorResults: SearchResult[] = corridors.map(corridor => ({
    id: corridor.id,
    type: 'corridor',
    title: `${corridor.source_asset} to ${corridor.destination_asset}`,
    subtitle:
      corridor.success_rate === undefined
        ? 'Payment corridor'
        : `${corridor.success_rate.toFixed(1)}% success rate`,
    searchText: `${corridor.source_asset} ${corridor.destination_asset}`,
  }));

  return [...anchorResults, ...corridorResults];
}

async function search(query: string): Promise<SearchResult[]> {
  const [anchorsResponse, corridorsResponse] = await Promise.allSettled([
    apiClient.get<ListResponse<AnchorRecord>>('/api/anchors?limit=100&offset=0'),
    apiClient.get<ListResponse<CorridorRecord>>('/api/corridors?limit=100&offset=0'),
  ]);

  if (anchorsResponse.status === 'rejected' && corridorsResponse.status === 'rejected') {
    throw new Error('Search sources are unavailable');
  }

  const anchors = anchorsResponse.status === 'fulfilled' ? anchorsResponse.value.data : [];
  const corridors = corridorsResponse.status === 'fulfilled' ? corridorsResponse.value.data : [];

  return filterSearchResults(buildSearchResults(anchors, corridors), query);
}

export function useSearchFunctionality(query: string) {
  const isOnline = useAppStore(state => state.isOnline);
  const normalizedQuery = query.trim();
  const canSearch = normalizedQuery.length >= 2;

  const queryState = useQuery({
    queryKey: ['mobile-search', normalizedQuery.toLowerCase()],
    queryFn: () => search(normalizedQuery),
    enabled: canSearch && isOnline,
    staleTime: 60 * 1000,
    retry: 1,
  });

  return {
    ...queryState,
    results: queryState.data ?? [],
    canSearch,
    isOnline,
  };
}
